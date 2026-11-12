# Port spec: color balance rgb

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/colorbalancergb.c` `commit_params()` and `process()`, with helpers from `src/common/colorspaces_inline_conversions.h` (Yrg, Ych, JzAzBz, `gamut_check_Yrg`, xyY, darktable UCS 22), `darktable_ucs_22_helpers.h` (`dt_UCS_22_build_gamut_LUT`, `lookup_gamut`, `soft_clip`), `chromatic_adaptation.h` (CAT16) and `math.h`. The OpenCL kernel is `colorbalancergb` in `data/kernels/extended.cl`, its UCS helpers in `data/kernels/colorspace.h`. The reference is darktable's CPU path.

Status: ported, both saturation formulas, in Slang (`crates/op/colorbalancergb`), as an operation with its manifest, schema and `prepare` (`ColorBalanceRgb`): darktable UCS, darktable's default for new edits since params version 5 and the schema's default (decision 22), and JzAzBz, darktable's older formula and its other choice. It is in `OPERATIONS` with `default_enabled` off, as darktable has color balance rgb off until the user turns it on: it is planned while its switched module is on, from a write of one of its controls to its reset (decision 32); Tool binding notes. Checked on one GPU so far.

Golden results (mire1.cr2, 3908×2602, Apple M4), through the operation's `prepare`:

| Case | darktable test | Formula | ΔE2000 mean | p95 | max | Pixels over 2.3 | One pass (median) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `colorbalancergb` | 0083, params version 4, every setting non-zero but `midtones_Y` | JzAzBz | 0.0057 | 0.0163 | 0.6501 | 0 | 3.6 to 3.8 ms |
| `colorbalancergb-ucs` | 0093-colorbalancergb-ucs, params version 5: chroma, saturation and brilliance per range, vibrance | darktable UCS | 0.0018 | 0.0049 | 0.0115 | 0 | 3.5 to 3.8 ms |

The module's effect on the second case, darktable's input against its output, is a mean ΔE2000 of 6.96, max 25.7.

## Purpose

Colour grading for the photographer: lift, gamma and gain by tonal range with colour wheels, chroma, saturation and brilliance per range, vibrance, a hue shift, and a contrast around middle grey. Algorithmically, luminance masks split the image into shadows, mid-tones and highlights; the wheels act in Filmlight's grading RGB as offset, slope and power; chroma in Filmlight Yrg; contrast as a power of Yrg luminance around a grey fulcrum; saturation and brilliance in darktable UCS 22 or JzAzBz, as the saturation formula says, then a gamut mapping to the working space in the same space. In the first pass the Colour balance RGB module, darktable's, drives it (decision 2).

## Pipeline

- **darktable position:** 41.5 in the v5.0 order, after color calibration (channelmixerrgb) and before RGB curve (42), RGB levels (43) and the tone mappers. The manifest's slot is working space 41.5, darktable's place: after the tone equalizer (camera RGB 24.0) and colour calibration (28.5), before RGB curve (42), RGB levels (43) and AgX (45.5), as darktable has them (decision 22; `docs/engine.md`, Order).
- **Domain:** scene-referred: it reads and writes linear light before the tone mapper.
- **Input and output:** RGBA in the working space, linear Rec2020, in and out; alpha passes through. Negative input is clipped to 0, and so is negative output.
- **Assumes earlier modules:** demosaic and the input profile; the gamut LUT and the matrices are the working space's.
- **Layer support:** yes (`layers: true`): per pixel, no statistics of the image.

## Operation

| Manifest field | Value |
| --- | --- |
| `id` | `colorbalancergb` |
| `version` | 1 |
| `slot` | working space, 41.5 (decision 22) |
| `domain` | scene |
| `layers` | true |
| `differentiable` | true: `colorbalancergb_backward` and `colorbalancergb_forward_diff` |
| `radius` | 0 |
| `provenance` | darktable, commit 20891f6c6f; `src/iop/colorbalancergb.c`, `src/common/colorspaces_inline_conversions.h`, `src/common/chromatic_adaptation.h`, `src/common/math.h`, `src/common/darktable_ucs_22_helpers.h`, `data/kernels/extended.cl` |

`prepare` gives one per-pixel pass of `main`, with the 272-byte `ColorBalanceParams` (`PARAMS_SIZE`) and the formula's gamut LUT as table 0 (binding 3, 512 floats). The kernel reads the formula from the uniform and takes one path or the other; both are in the same entry points. It ignores the image context: nothing in darktable's `commit_params` reads the image.

## Parameters

darktable params version 5, 132 bytes: 32 floats and an enum, in this order. The schema (version 1) has darktable's field names, case kept, its ranges (the introspection `$MIN` and `$MAX`) and its defaults, `saturation_formula`'s darktable UCS included (decision 22). darktable has no `reload_defaults` for the module, so its introspection defaults are its scene-referred defaults; `Settings::default` holds them, and a test checks the schema against it.

| Name | Type | Range | Default | Meaning |
| --- | --- | --- | --- | --- |
| `shadows_Y`, `midtones_Y`, `highlights_Y`, `global_Y` | float | −1 to 1 | 0 | Luminance of each wheel: shadows' and highlights' slope, mid-tones' power, global offset |
| `shadows_C`, `midtones_C`, `highlights_C`, `global_C` | float | 0 to 1 | 0 | Chroma of each wheel |
| `shadows_H`, `midtones_H`, `highlights_H`, `global_H` | float, degrees | 0 to 360 | 0 | Hue of each wheel, on the conventional wheel (Yrg less 30°) |
| `shadows_weight`, `highlights_weight` | float | 0 to 3 | 1 | Fall-off of the shadows and highlights masks |
| `white_fulcrum` | float, EV | −16 to 16 | 0 | White the mid-tones power is normalised to, `2^white_fulcrum` |
| `chroma_shadows`, `chroma_midtones`, `chroma_highlights` | float | −1 to 1 | 0 | Chroma per range |
| `chroma_global` | float | −1 to 1 | 0 | Chroma everywhere |
| `saturation_global` | float | −1 to 1 | 0 | JzAzBz saturation everywhere |
| `saturation_shadows`, `saturation_midtones`, `saturation_highlights` | float | −1 to 1 | 0 | Saturation per range |
| `hue_angle` | float, degrees | −180 to 180 | 0 | Rotates every hue |
| `brilliance_global`, `brilliance_shadows`, `brilliance_midtones`, `brilliance_highlights` | float | −1 to 1 | 0 | JzAzBz brilliance: lightness along the saturation axis |
| `mask_grey_fulcrum` | float | 0 to 1 | 0.1845 | Middle grey of the masks, as Yrg luminance |
| `vibrance` | float | −1 to 1 | 0 | Chroma, mostly on low-chroma colours |
| `grey_fulcrum` | float | 0 to 1 | 0.1845 | The luminance contrast keeps fixed, as Yrg Y |
| `contrast` | float | −1 to 1 | 0 | Luminance power around the grey fulcrum, `1 + contrast` |
| `saturation_formula` | choice | `jzazbz`, `ucs` | `ucs` | The space of brilliance, saturation and the gamut mapping: darktable UCS (2022), or JzAzBz (2021) |

darktable shows most of these as percentages and gives some soft ranges, the slider's span within the hard range: `contrast` ±0.5, `grey_fulcrum` 0.1 to 0.5, `vibrance` and `chroma_global` ±0.5, `white_fulcrum` ±2 EV, `global_Y` ±0.05, `global_C` 0 to 0.01, `shadows_C` 0 to 0.5, `midtones_Y` ±0.25, `midtones_C` 0 to 0.1, `highlights_Y` ±0.5, `highlights_C` 0 to 0.2; `shadows_Y`'s soft range is its hard one, ±1. The schema's ranges are the hard ones; a slider's span is the binding's.

In the first pass the Colour balance RGB module owns every parameter (Tool binding notes).

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 | 24 floats, up to `hue_angle` | Defaults for the rest, `saturation_global / 100`, JzAzBz |
| 2 | Appends the four brilliances | Defaults for the rest, JzAzBz |
| 3 | Appends `mask_grey_fulcrum` | Defaults for the rest, JzAzBz |
| 4 | Appends `vibrance`, `grey_fulcrum`, `contrast` | JzAzBz |
| 5 | Appends `saturation_formula` (default darktable UCS) | Current |

The defaults filled in are 0, but 1 for the fall-offs and 0.1845 for the fulcrums. `ports.rs` migrates every version as darktable does; the golden case is version 4. A test covers version 1's division of the global saturation by 100, which no case reaches.

## Setup on the CPU (`commit_params`)

- `contrast' = 1 + contrast`; `hue_angle'` in radians.
- `RGB_norm` = grading RGB of Ych (1, 0, h = 0).
- For each of global, shadows, highlights, midtones: `Ych = (1, C, H + ANGLE_SHIFT)` in radians, with `ANGLE_SHIFT = -30°`, converted to grading RGB, then:
  - global: `G = (rgb - RGB_norm) + RGB_norm · global_Y`
  - shadows: `S = 1 + (rgb - RGB_norm) + shadows_Y`
  - highlights: `H = 1 + (rgb - RGB_norm) + highlights_Y`
  - midtones: `M = 1 / (1 + (rgb - RGB_norm))`; `midtones_Y' = 1 / (1 + midtones_Y)`
- `shadows_weight' = 2 + 2 · shadows_weight`, likewise highlights; `midtones_weight = s² h² / (s² + h²)` from the primed weights.
- `white_fulcrum' = 2^white_fulcrum`; `mask_grey_fulcrum' = mask_grey_fulcrum^0.4101205819200422`.
- `L_white = Y_to_dt_UCS_L_star(white_fulcrum')`, the darktable UCS lightness of the white fulcrum, which darktable computes in `process`; the JzAzBz path ignores it.
- Gamut LUT, 512 entries over hue in [-π, π), the formula's:
  - JzAzBz: for every RGB on a 92³ grid in [0, 1] of the working space, convert to JzAzBz and keep the largest Cz/Jz per hue bin, then a circular 5-tap box average.
  - darktable UCS (`dt_UCS_22_build_gamut_LUT`, in `setup.rs`): the working primaries' chromaticities in CIE xy (through `dt_D65_XYZ_to_xyY`, so negative XYZ clipped first), and their angles around D65 (0.31271, 0.32902). Then 25,600 rays from D65, angle `−π + i / 25600 · 2π`: the edge of the primaries' triangle each crosses, found by which pair of primaries' angles the ray's lies between (`Delta_H`, differences wrapped to [−π, π]), and the crossing point. Each point goes to darktable UCS U*′V*′ (`xyY_to_dt_UCS_UV`); its hue `atan2(V*′, U*′)` rounds to an entry as the JzAzBz table's does (`round(511 · (h + π) / 2π)`, wrapped), and the entry is the mean of its points' M² = U*′² + V*′², the boundary's colourfulness squared; an entry no point reaches is 0, which on linear Rec2020 none is (a unit test).

Working-space matrices: darktable builds linear Rec2020 in lcms from primaries R (0.708, 0.292), G (0.170, 0.797), B (0.131, 0.046) and white (0.31271, 0.32902), Bradford-adapted to D50 (0.9642, 1, 0.8249). The output matrix is its inverse. The module premultiplies `input = LMS2006 · CAT16(D50→D65) · RGB_to_XYZ` and `output = XYZ_to_RGB · CAT16(D65→D50)`.

**The gamut LUT is built once for each formula.** darktable builds it again only when the working profile or the saturation formula changes (`lut_inited`). `gamut_lut` keeps linear Rec2020's, Apertrust's only working space, in a `OnceLock` per formula, so `prepare` costs microseconds on every slider drag after the first (Choices, 3). The darktable UCS table takes far less to build than the JzAzBz one, 25,600 points against 92³. Measured on the Apple M4's CPU, 1000 calls of `ColorBalanceRgb.prepare` with the contrast moving: the first 42 to 51 ms, the table; then a median of 4.9 µs and at most 31 µs in a release build, 243 ms and 72 µs in a debug one. A unit test checks that the table is built once (the same borrowed slice on every call).

**Divisions by zero are guarded** (Choices, 4): `grey_fulcrum`, `mask_grey_fulcrum` and `1 + midtones_Y` are at least 1e-6 where they divide, and each channel's `1 + (rgb - RGB_norm)` of the midtones wheel is at least 1e-6 from 0, keeping its sign. darktable's ranges reach 0 for the fulcrums, −1 for `midtones_Y`, and a 0 of the wheel's `1 + v` from a chroma of about 0.31 at some hues, where darktable divides by zero. So every exponent the kernel gets is within ±10⁶, which its bounded power keeps finite (Precision).

## Per pixel (`process`)

Steps 1 to 5 and 8 are the same for both formulas; steps 6 and 7 are the formula's, darktable UCS's by default.

1. Clip negative RGB; `LMS = input · RGB`; LMS → Yrg → Ych; `Y = max(Y, 0)`.
2. Masks from `x = Y^0.4101205819200422`: shadows `α = 1/(1+e^{(x-f)/f · s})`, highlights `β = 1/(1+e^{-(x-f)/f · h})`, midtones `γ = e^{-(x-f)² · m/4} · (1-α)² (1-β)² · 8`, with `f` the mask fulcrum.
3. Rotate hue by `hue_angle`. Chroma × `max(1 + chroma_global + masks·chroma + vibrance · (1 - c^|vibrance|), 0)`. Clip chroma to the Yrg gamut (`gamut_check_Yrg`).
4. Ych → Yrg → LMS → grading RGB. `RGB += G`; `RGB *= (1-β)((1-α) + α·S) + β·H`; per channel `RGB = sign · (|RGB|/w)^M · w`, with `w` the white fulcrum.
5. Grading RGB → LMS → Yrg. `Y = max(Y/w, 0)^midtones_Y' · w`; `Y = g · (Y/g)^contrast'`, with `g` the grey fulcrum. Then Yrg → LMS → XYZ D65.
6. Brilliance and saturation:
   - **darktable UCS:** XYZ D65 → xyY (`dt_D65_XYZ_to_xyY`, negative XYZ clipped, black at D65's chromaticity) → JCH (`xyY_to_dt_UCS_JCH`, with `L_white`) → HCB, brightness `B = J (C^1.33654 + 1)`. Rotate (C, B) by its angle: `sin T = C / r`, `cos T = B / r` with `r = hypot(C, B)` (both 0 for black), `P = max(C, FLT_MIN)` and `W = C sin T + B cos T`, which is `r`. Saturation `a = max(1 + saturation_global + masks·saturation, 0)`, soft-clipped at 0.5–1 × `max_a = hypot(P, W) / P` so the root below stays real, and brilliance `b = max(1 + brilliance_global + masks·brilliance, 0)`. Then `P′ = (a − 1) P`, `W′ = sqrt(P² (1 − a²) + W²) · b`, and rotate back: `C = max(P′ cos T + W′ sin T, 0)`, `B = max(−P′ sin T + W′ cos T, 0)`. HCB → JCH.
   - **JzAzBz:** XYZ → JzAzBz, then Jz, Cz and hue `h`. Rotate (Jz, Cz) by `T = atan2(Cz, Jz)` to (S, O): `O = S · clamp(T · sat, -T, π/2 - T)`, `S = max(S · bright, 0)`, with `sat = saturation_global + masks·saturation` and `bright = 1 + brilliance_global + masks·brilliance`. Rotate back, clamped at 0.
7. Gamut mapping:
   - **darktable UCS:** the boundary's chroma at this hue and lightness, `15.933 (J L_white)^0.6524 · LUT(H)^0.6008 / L_white` (the table holds M²), as a saturation at its brightness (JCH → HSB). The pixel's saturation `S = C / B` (0 where B is 0) soft-clipped at 0.8–1 × the boundary's, at constant brightness and hue. HSB → JCH → xyY (`dt_UCS_JCH_to_xyY`, L* clipped to [0, 2.09885]) → XYZ D65 (`dt_xyY_to_XYZ`, black where y is 0).
   - **JzAzBz:** `sat = Cz/Jz` soft-clipped at 0.8–1 × LUT(h). Average (Jz, Cz) with the point at that saturation, which keeps that saturation and moves lightness and chroma together. Then clip Cz so that L′M′S′ stays non-negative at constant Iz and hue. JzAzBz → XYZ D65.
8. `RGB = output · XYZ`; clip negatives.

**At the defaults the module is not an identity**, with either formula. Steps 1, 3's Yrg clip, 7 and 8 run whatever the settings.

Colours within about 0.8 of the working gamut's saturation at their hue pass unchanged (within 1e-4 on the M4 with JzAzBz; with darktable UCS within a ΔE2000 of 0.0005 on the sweep, as Exposure's row E1 measured it), but more saturated ones are pulled inside and negative channels clipped. With JzAzBz a pure Rec2020 red (0.5, 0, 0) comes out (0.508, 0.015, 0.011).

On the case's input, darktable's render of mire1.cr2 before the module, the JzAzBz defaults change ΔE2000 by mean 0.0118, p95 0.0112, max 2.39, with 0.25% of pixels over 1 and 0.66% over 0.5: the chart's saturated patches. darktable UCS's defaults, as darktable renders them on that input, change it by mean 0.009, p95 0.005, max 3.28, and the port's defaults match darktable's within a mean ΔE2000 of 0.0016, max 0.0106 (a local check against darktable, not a case).

Both clip a colour on the gamut's boundary to 0.926 of its saturation; their boundaries differ (the largest JzAzBz saturation over the RGB cube, against the primaries' triangle in darktable UCS at the pixel's lightness), and darktable UCS keeps brightness where JzAzBz moves lightness and chroma together.

So a module's first step, which adds the operation, moves colours the control does not name, as darktable's first step does (`docs/modules/colour-balance-rgb.md`, B1: with darktable UCS, after colour calibration, means of 0.0000 to 0.021 ΔE2000 on the corpus and single pixels up to 3.2). An unedited photo, like darktable's unedited render, gets no gamut mapping (decision 32). The Colour balance RGB module's spec reports the first step's shift as its row B1; a module has no tooltip of its own to mention it in yet.

## Neighbourhood and scale

- **Neighbourhood radius:** none; a per-pixel module.
- **Scale dependence:** none.
- **Tile overlap needed:** none.
- **Region-of-interest changes:** none.

## Precision and special values

- **fp64:** none in darktable's processing path. The port computes the matrices in f64 and rounds them to f32 once.
- **The grey fulcrum is Yrg luminance**, CIE 2006 LMS's, not working-space RGB: working-space white (1, 1, 1) has Yrg Y 1.0579. The grey contrast keeps fixed at the default fulcrum 0.1845 is RGB (0.1744, 0.1744, 0.1744); RGB 0.18 has Y 0.1904.
- **The settings' powers overflow within darktable's ranges.** The midtones wheel's `1 + v` crosses 0 from a chroma of about 0.31 at some hues (its range is 0 to 1): chroma 0.4 at 180.02° gives an exponent of −2.8·10⁶, 0.6 at 27.34° one of −19831, and a negative exponent sends dark channels to infinity. `midtones_Y` near −1 gives an exponent up to 10⁶, which sends luminance above the white fulcrum to infinity. In darktable the pixel then becomes NaN or black; on the GPU, what follows an infinity is implementation-defined. The setup keeps every exponent within ±10⁶ (Setup), and the kernel computes the three powers whose exponents come from the settings (each channel's midtones power, the midtones luminance power and contrast) as `bounded_pow(x, y) = 2^min(y·log₂ x, 32)` for `x > 0`, else 0. That is finite for any exponent. Below 2^32 times the white or grey fulcrum it equals `pow`, so neither the golden case nor the gradients reach it. The cap lies far past JzAzBz's pole (next item), so it only changes pixels whose output is already degenerate; it is that low because the derivatives square such values (Yrg's normalisation divides by its sum squared), and at 2^64 they overflow. A GPU test sweeps the wheel's hue at chroma 0.4 to 1 and checks the forward derivatives too, which are not clipped as the output is, so they carry an infinity or NaN out of the kernel: with `pow0` both of its finite-output tests fail.
- **JzAzBz's pole:** `Jz = (1 + d)·Iz / (1 + d·Iz) − d0` with `d = −0.56` grows without bound as `Iz` nears 1/0.56, which a grey reaches at Y about 8.7·10⁵ (19.7 EV over white); past it the formula turns negative and the clip at 0 makes Jz 0. darktable runs the same fp32 maths. So a grey at RGB 1000 (10 EV over white) with contrast +1 reaches Y 6·10⁶ and comes out near black, while at +0.5 it reaches 8·10⁴ and comes out right. A pixel landing within rounding of the pole itself would get an infinite Jz; the colour library's `xyz_to_jzazbz` does not guard it yet, nor does darktable. Planned: `xyz_to_jzazbz` in `apertrust-color` guarded where `1 + d·Iz` nears 0, with a test at the pole. It follows the kernel rules and changes no render darktable gives a finite value for.
- **`pow` at zero:** `pow0(0, y)` and `bounded_pow(0, y)` are 0 for every `y`. At `contrast` −1 the kernel's exponent is 0: darktable's `powf(0, 0) = 1` sends black to the fulcrum, the kernel keeps black at 0; every other luminance becomes the fulcrum on both sides. With a negative midtones exponent, darktable's `powf(0, y)` is infinite and the kernel's 0.
- **darktable UCS's divisions:** darktable guards three of them, the chromaticity's two denominators by `FLT_MIN` with their sign and `dt_UCS_JCH_to_xyY`'s by clipping L* to 2.09885, just under the range 2.0988838 where `dt_UCS_L_star_to_Y` would divide by zero, so the brightest lightness comes back as Y about 4.5·10⁷; the colour library's `ucs_*` functions do the same. Two more the port guards: the chroma the saturation's soft clip divides by is at least 10⁻¹⁵ where darktable's is `FLT_MIN`, since for a grey with darktable's `P = FLT_MIN` the ratio `W / P` overflows from a brightness of 4 (white fulcrum 0 reaches 2.1, at −16 EV 1265) and its derivative divides by `P²`, which underflows; and the square root's argument is held at 0 or above, where rounding could make it negative as `a` nears `max_a` (Deliberate differences).
- **darktable UCS's lightness** tends to 2.0988838 as Y grows, so very bright pixels lose lightness resolution in fp32: Y 10⁴ is L* 2.0919, 2.4·10⁻⁷ apart per ulp, and Y comes back from it within 10⁻⁴ relative (a unit test). darktable runs the same fp32 maths. Its OpenCL kernel also clips Y to 10⁸ before the lightness (CPU and OpenCL differences).
- **NaN and infinity:** darktable does not check for them. The kernel relies on neither: negative input is clipped, exponentials are clamped (`logistic`), the guarded divisors keep the uniforms finite and `bounded_pow` keeps the powers finite. What remains is JzAzBz's pole, above; darktable UCS has none within fp32's range, and its output and forward derivatives stay finite on the GPU tests' extreme inputs and settings.

## CPU and OpenCL differences in darktable

Not compared line by line. The contrast and gamut-mapping steps of `colorbalancergb` in `extended.cl` match the CPU path, its `dtcl_pow` for `powf`. In the darktable UCS path, `xyY_to_dt_UCS_JCH` in `colorspace.h` clips Y to [0, 10⁸] (`DT_UCS_Y_UPPER_LIMIT`) before converting it to L*, where the CPU path does not; it only changes pixels over 10⁸, whose L* is within fp32's rounding of the range either way. The port follows the CPU path. The references are rendered on the CPU.

## Deliberate differences

- **Divisions by zero guarded** at 1e-6 (Setup), where darktable's ranges reach them, the midtones wheel's included.
- **darktable UCS's saturation divides by a chroma of at least 10⁻¹⁵**, where darktable's floor is `FLT_MIN` (Precision). It changes only colours whose UCS chroma is under 10⁻¹⁵, whose output chroma then moves by at most about 10⁻¹⁵: nothing measurable. **Its square root's argument is held at 0 or above**, which only rounding could take below.
- **The darktable UCS gamut LUT's entries are summed in order**, where darktable's OpenMP threads share each entry's sum: a difference in the last bits of an entry's mean.
- **The settings' powers bounded** at 2^32 in the log domain (`bounded_pow`, Precision), where darktable's `powf` overflows to infinity.
- No mask display: the checkerboard preview is a GUI feature, not image processing.
- `hypot(x, y)` is `sqrt(x² + y²)`, as darktable's own `dt_fast_hypotf` is in its release builds. That only loses precision for values far outside image data.
- darktable's per-channel midtones power uses a polynomial `log2`/`exp2` approximation; the GPU uses its own `exp2` and `log2`.
- Matrices are computed in f64 and rounded to f32; darktable inverts the output matrix in f32.
- `pow` with a zero exponent is guarded, so vibrance 0 never computes `0^0` on the GPU, where it is undefined; and `pow0(0, 0)` is 0 (Precision).
- The gamut LUT lookup, both formulas', always interpolates between entry `i` and `i + 1`. darktable's returns entry `i` alone when the hue falls exactly on it, which gives the same value, but in Slang's backward pass that branch can read a neighbour the forward pass never saved.

## Choices

1. **Parameter names keep darktable's case**: `shadows_Y`, `shadows_C`, `shadows_H` and the other wheels' `_Y`, `_C`, `_H`, as darktable's struct and its introspection spell them; the other fields are lower case already. They are not lower-cased (`shadows_y`) to match the Rust fields and the other operations' all-lower-case names on the CLI.
2. **Choice names from darktable's descriptions**, as AgX's and RGB curve's are: `jzazbz` ("JzAzBz (2021)") and `ucs` ("darktable UCS (2022)", shown as "UCS (2022)"). Not the enum's suffixes (`jzazbz`, `dtucs`).
3. **The gamut LUT cached in a `OnceLock` per formula, for linear Rec2020**, Apertrust's only working space; any other profile, which only tests pass, builds its table on each call. Not a cache keyed by profile (a mutex over leaked tables, for working spaces Apertrust does not have), nor caching `prepare`'s output in the engine, nor building the table on the GPU. `prepare` still encodes the 2 KB table into bytes on each call, about a microsecond.
4. **Divisions by zero floored at 1e-6 in `prepare`**, keeping the sign of the midtones wheel's `1 + v`, the schema keeping darktable's ranges. Narrowing the schema's minimums (a grey fulcrum of 0.001, say) would set ranges darktable does not have and would not cover the wheel, whose pole depends on chroma and hue together; passing the values through as darktable does sends infinities or NaN to the kernel.
5. **Provenance lists the source files whose code is ported**: `darktable_ucs_22_helpers.h` since darktable UCS is ported, not `gamut_mapping.h`, which only the GUI uses.
6. **The golden case goes through the operation** with a nominal 1×1 image: `ports.rs` turns the sidecar's settings into parameter values, resolves them against the schema and calls `ColorBalanceRgb.prepare`. Its inputs equal the setup's byte for byte (a test).
7. **The settings' powers bounded in the kernel**, at 2^32 in the log domain (Precision), besides the setup's bounded exponents. The alternatives fall short: bounding the exponents alone, since any exponent above 1 or below 0 overflows for bright or dark enough pixels; clamping each power's input, which needs a bound per exponent in the uniforms; a cap at 2^64, at which the derivatives overflow; clamping the output, which cannot see an infinity inside the kernel.
8. **One kernel for both formulas**, the uniform's `saturation_formula` choosing the path, as darktable's `process` does. A branch on a uniform takes the same side for every pixel and in the backward pass's recomputation, so it costs nothing measurable (3.5 to 3.8 ms a pass for either case on the M4) and changes neither path's arithmetic. An entry point per formula would double the derivative entry points and the kernels the engine compiles for one operation.
9. **The UCS conversions in the colour library, the UCS gamut table in the operation's setup**: `ucs.rs` and `color.slang`'s `ucs_*` functions are darktable's shared conversions (`colorspaces_inline_conversions.h`), which darktable's color equalizer and color harmonizer use too, and `ucs_gamut_lut` is in `setup.rs` beside `prepare`. The table is not in the colour library beside `jzazbz_gamut_lut`: `dt_UCS_22_build_gamut_LUT` has one user, and shared helpers stay in the port until a second port needs them (decision 15's rule).
10. **The floor of the UCS saturation's chroma at 10⁻¹⁵** (Precision): the smallest power of ten whose square, 10⁻³⁰, keeps the derivative `W / P²` finite for every brightness fp32 reaches here (at most about 1265, at the white fulcrum's −16 EV), with no measurable change in output. Not darktable's `FLT_MIN`, whose ratio overflows past a brightness of 4 and whose derivative divides by an underflowed `P²`, giving infinities and NaN in the backward pass; nor flooring only the ratio's divisor, which leaves `max_a` near 10⁻²³ for black and the soft clip's derivatives overflowing.

## Tool binding notes

In the first pass (decision 2) the Colour balance RGB module is darktable's and owns every parameter, each as darktable shows it, `contrast` as darktable's percentage and `grey_fulcrum` at its 18.45 % (decision 22): its master, 4 ways and masks tabs are content tabs (decision 29), and its contrast, grey fulcrum and power luminance, `contrast`, `grey_fulcrum` and `midtones_Y`, are its controls as every other parameter is (`docs/modules/README.md`). It is built (`crates/modules/src/colour_balance_rgb.rs`, `docs/modules/colour-balance-rgb.md`), each percentage the parameter times 100.

What the binding needs:

- **Contrast** is `contrast`, a float, −1 to 1, default 0; darktable's slider spans ±0.5 and shows ±50%. The kernel applies `Y' = g · (Y/g)^(1 + contrast)` to Yrg luminance at constant Yrg chromaticity (r, g): saturation as Yrg measures it does not change, and greys stay grey (GPU tests, within 2e-4 in r and g).
- **Pivot** is `grey_fulcrum`, a float, 0 to 1, default 0.1845; darktable's slider spans 0.1 to 0.5 and shows a percentage. It is Yrg luminance: the default keeps RGB grey 0.1744 fixed (Precision).
- **Contrast 0 is a no-op of the contrast** (the kernel gets an exponent of exactly 1), **but not of the operation**: its gamut mapping runs at every setting (Per pixel). The module is switched (decision 32): off, the operation is not planned and the frame is byte-identical to no edit. Its first step, on at its defaults, adds the operation and its gamut mapping, as darktable's does, measured as the module's row B1 without a bound (decision 22).
- **The first step with darktable UCS**, the default (decision 22), is the module's row B1 (`docs/modules/colour-balance-rgb.md`): the frame's ΔE2000 against no edit, measured on CI's synthetic images and locally on the chart and the corpus, and reported without a bound. On the M4, with colour calibration on every colour raw (decision 27), which compresses the gamut first:
  - greys and the sweep's colours within 80% of the gamut's saturation: 0.000;
  - inside the gamut: a mean of 0.061, 2.2% over 1.0, max 3.9;
  - the whole sweep: 0.148, 5.9%, max 4.0;
  - the chart: 0.0069, 0.021%, max 2.3;
  - the 31 corpus raws: means of 0.0000 to 0.021, at most 0.24% over 1.0, single pixels up to 3.2.
- **The pivot's untouched value is its default** (decision 28, for a control whose unit has no natural zero): `grey_fulcrum` at 0.1845 with contrast 0 sets nothing.
- **One module, one operation:** the Colour balance RGB module owns every parameter, so none can be set at operation level (`Set`, `--param`; decision 28); the operation is planned while the module is on, as one instance (decision 32).
- **Derivatives:** `contrast` is the second of the kernel's four gradient slots; `grey_fulcrum` is not differentiated.
- **Saturation formula:** `saturation_formula`, darktable's "saturation formula" on its Masks tab, an ordinary content tab (decision 29); darktable UCS by default, JzAzBz the other choice (decision 22). darktable's presets set it: "basic colorfulness | legacy" JzAzBz, "natural skin", "vibrant colors" and "standard" darktable UCS, all of which the port renders.

## Gradients

The Slang kernel differentiates against `chroma_global`, `contrast` (as `1 + contrast`, with the same derivative), `saturation_global` and `brilliance_global`, in that order, with either formula: darktable UCS's saturation `a` and brilliance `b` take the same two slots as JzAzBz's. Apple M4, step 1e-3, the sum of output RGB as the loss, through the operation's `prepare`. The synthetic sweep runs each case's settings, so each formula, on the hue, saturation and level sweep.

| Case | Parameter | Case: relative error | Case: forward vs reverse | Synthetic: relative error | Synthetic: forward vs reverse |
| --- | --- | --- | --- | --- | --- |
| `colorbalancergb` (JzAzBz) | `chroma_global` | 2.6e-3 | 4.5e-6 | 9.8e-4 | 5.8e-6 |
| | `contrast` | 4.8e-5 | 1.0e-6 | 2.6e-4 | 2.0e-6 |
| | `saturation_global` | 2.3e-6 | 4.2e-6 | 1.7e-4 | 7.2e-6 |
| | `brilliance_global` | 1.3e-5 | 2.8e-7 | 1.4e-4 | 7.1e-7 |
| `colorbalancergb-ucs` (darktable UCS) | `chroma_global` | 3.0e-5 | 2.9e-6 | 7.3e-4 | 3.2e-6 |
| | `contrast` | 1.7e-5 | 5.1e-7 | 1.0e-5 | 6.9e-7 |
| | `saturation_global` | 7.6e-5 | 4.5e-6 | 9.9e-5 | 2.4e-6 |
| | `brilliance_global` | 1.8e-5 | 4.8e-7 | 2.3e-5 | 6.9e-7 |

`grey_fulcrum` has no slot of the four. When an optimiser first needs its derivative, it takes the slot substitution RGB curve uses.

Known non-smooth spots, where finite differences straddle a kink and per-pixel gradients jump: the midtones power at zero in each grading-RGB channel, the LUT's nodes, the clips (negative RGB, the Yrg gamut, the final negative clip, and JzAzBz's L′M′S′ chroma clip or darktable UCS's clips of C and B at 0 and of L* at 2.09885), the saturation's and brilliance's floors at 0, darktable UCS's chromaticity's `abs`, and `bounded_pow`'s cap, past which its derivative is 0.

## Test plan

- **Golden case:** `colorbalancergb`, test 0083 on mire1.cr2, params version 4, which darktable's `legacy_params` and `ports.rs` give JzAzBz whatever the schema's default, every setting non-zero but `midtones_Y`, `filmicrgb` disabled. `Tolerance::DARKTABLE`.
- **Golden case for darktable UCS** (decision 22): `colorbalancergb-ucs`, darktable's test 0093-colorbalancergb-ucs on mire1.cr2, params version 5 with darktable UCS, unchanged, `filmicrgb` disabled, `Tolerance::DARKTABLE`: darktable's own integration test of the formula. darktable's tests have no case of darktable UCS at its defaults; a local check against darktable matched them (Per pixel). Planned: a golden case of the UCS formula at its defaults.
- **Unit tests** (`crates/op/colorbalancergb`): the schema's defaults are darktable's, darktable UCS included; the schema is version 5's 33 fields in order; contrast's and the pivot's ranges; every parameter reaches its own field and round-trips through `values`; `prepare` gives one per-pixel pass of 272 bytes with the formula's gamut LUT, equal to the setup's; each formula's LUT is built once and differs from the other's, and another working space gets tables of its own, lower at most hues; the darktable UCS LUT reaches every hue of Rec2020; the formula and `L_white` reach the uniform's last two words; the guarded divisors give finite uniforms; the midtones wheel's exponents stay within ±10⁶ at every tenth of a degree for chroma 0.31 to 1, and are negative at 180.02° for 0.4 and 27.34° for 0.6; the manifest agrees with the kernel about derivatives.
- **In `ports.rs`:** each case's inputs through the operation equal the setup's, and every gradient slot moves them; test 0083 reads as version 4 with JzAzBz and test 0093 as version 5 with darktable UCS and its settings; version 1 divides the global saturation by 100.
- **The whole pipeline:** not in it, since neither darktable nor Apertrust runs it on an unedited photo (decision 32).
- **On the GPU** (`crates/golden/tests/colorbalancergb.rs`, through `prepare`, run in CI), each with both formulas: the defaults leave greys and colours inside the gamut unchanged within 5e-4, from 1e-3 to 4; they still move pure primaries and clip negatives; contrast raises Yrg luminance to its power around the fulcrum at constant chromaticity, for four contrasts and fulcrums; the grey at the fulcrum stays put from contrast −0.9 to 1. Output and forward derivatives are finite for black, tiny, negative, very bright and very saturated input at the defaults, contrast ±1, fulcrums 0 and 1, the mask fulcrum 0, `midtones_Y` −1 (with the white fulcrum at −16 EV too), the colour controls at their maximum and every control at either end of its range; and for colours and greys across the midtones wheel's hue every 3° at chroma 0.4, 0.6, 0.8 and 1, plus the two poles above.
