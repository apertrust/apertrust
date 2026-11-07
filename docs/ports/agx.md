# Port spec: AgX

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/agx.c` `commit_params()` (`_calculate_tone_mapping_params`, `_get_primaries_params`) and `process()` (`_create_matrices`, `_compress_into_gamut`, `_agx_tone_mapping`), with helpers from `src/common/custom_primaries.c` (`dt_rotate_and_scale_primary`), `src/common/colorspaces.c` (`dt_make_transposed_matrices_from_primaries_and_whitepoint`, a profile's primaries and white), `src/common/colorspaces_inline_conversions.h` (HSV), `src/common/matrices.c` (`mat3SSEinv`) and `src/common/iop_profile.c`. OpenCL: `data/kernels/agx.cl`. The reference is darktable's CPU path.

Status: ported.

- **Code:** the kernel is `slang/agx.slang` in `crates/op/agx`, its setup `src/setup.rs`, with shared helpers in `apertrust-color` (`src/lib.rs` and `slang/color.slang`).
- **Coverage:** every setting of params version 7 except base primaries "export profile", which returns `NotPorted`.
- **Checks:** all four golden cases pass, and the gradient check passes on every case and on the synthetic sweep (Gradients). Checked on one GPU so far.
- **Module:** AgX, darktable's page in the Base tab, owning every parameter ([../modules/agx.md](../modules/agx.md)). A module set to each golden case's values binds its parameters bit for bit but three of `agx`'s, one f32 step off.

Golden result, Apple M4:

| Case | darktable test | Image | ΔE2000 mean | p95 | max | Module effect (mean) |
| --- | --- | --- | --- | --- | --- | --- |
| `agx` | 0175 | mire1.cr2, 3908×2602 | 0.0000 | 0.0001 | 0.0003 | 7.02 |
| `agx-default` | 0174 | mire1.cr2, 3908×2602 | 0.0000 | 0.0001 | 0.0003 | 3.19 |
| `agx-highlights` | 0186 | hlrecovery.arw, 5492×3672 | 0.0000 | 0.0001 | 0.0003 | 4.86 |
| `agx-edges` | 0174, edited | mire1.cr2, 3908×2602 | 0.0000 | 0.0001 | 0.0005 | 6.40 |

One pass takes 3.6 to 3.7 ms on a 10 MP image and 7.4 ms on 20 MP (median), the memory-bound cost of any full-image pass on the M4.

## Purpose

AgX is the tone mapper: it turns scene-referred light, unbounded and with mid-grey at 0.18, into display-referred values, and gives bright saturated colours a gradual path to white instead of clipping into a flat primary.

It encodes each RGB channel in log2 around mid-grey and maps it through a sigmoid-like curve with a linear section around a pivot. It does so in a "rendering" RGB space whose primaries are inset towards white and rotated, so the per-channel curve desaturates bright colours. After the curve, a second set of primaries restores purity and undoes the rotation, and the input's hue can be partly restored.

In Apertrust it is the background's tone mapper, run once (architecture decision 33), and in the first pass (decision 2) a module of its own, AgX, the tone mapper in the Base tab, one tone mapper per edit.

## Pipeline

- **darktable position:** after color balance rgb, rgb curve, rgb levels, base curve, filmic and sigmoid; before filmic rgb, colisa, tone curve and the display-referred modules (iop order v5.0, `src/common/iop_order.c`). In Apertrust it runs where darktable runs it: after exposure, the tone equalizer, colour calibration, color balance rgb, rgb curve (working 42) and rgb levels (working 43) and all their layer instances, and before the display-referred effects and output colour (decision 22; `docs/engine.md`, Order). Its slot is darktable's, working space 45.5.
- **Domain:** scene-referred in, display-referred out.
- **Input and output:** working RGB, linear Rec2020 in and out. The output is linear, with display white at 1.0; the look and the outset matrix can move it slightly outside [0, 1], and nothing here clips it.
- **Assumes earlier modules:** demosaic, white balance or colour calibration, and exposure placing mid-grey at 0.18: the log encoding is relative to 0.18. Input may be negative (out-of-gamut camera colours); `_compress_into_gamut` handles that.
- **Layer support:** background only until layers come. The background's tone mapper runs once and its parameters are global; layers change the light it receives. Planned with layers: a layer may choose its own tone mapper, AgX among them, which replaces the background's within its mask, never stacked on it, so a pixel is tone-mapped once (decision 33).

## Parameters

darktable params version 7, 144 bytes, 36 four-byte fields in this order: `look_lift`, `look_slope`, `look_brightness`, `look_saturation`, `look_original_hue_mix_ratio`, `range_black_relative_ev`, `range_white_relative_ev`, `dynamic_range_scaling`, `curve_pivot_x`, `curve_pivot_y_linear_output`, `curve_contrast_around_pivot`, `curve_linear_ratio_below_pivot`, `curve_linear_ratio_above_pivot`, `curve_toe_power`, `curve_shoulder_power`, `curve_gamma` (f32), `auto_gamma` (gboolean, i32), `curve_target_display_black_ratio`, `curve_target_display_white_ratio` (f32), `base_primaries` (enum, i32), `disable_primaries_adjustments` (i32), `red_inset`, `red_rotation`, `green_inset`, `green_rotation`, `blue_inset`, `blue_rotation`, `master_outset_ratio`, `master_unrotation_ratio`, `red_outset`, `red_unrotation`, `green_outset`, `green_unrotation`, `blue_outset`, `blue_unrotation` (f32), `completely_reverse_primaries` (i32).

Defaults are darktable's scene-referred defaults (`reload_defaults` calls `_set_scene_referred_default_params` in a scene-referred workflow; `agx::Settings::scene_referred_default`, checked field for field against test 0186's stored params); where the introspection `$DEFAULT` differs, it is given in brackets. The AgX module, the tone mapper of its own in the Base tab, owns every parameter, one control each, as darktable's Settings and Primaries tabs show them, its show curve, advanced curve parameters and look sections collapsible groups, and `dynamic_range_scaling` a control on the Settings tab, not ported, as darktable shows it (decision 29; `docs/modules/README.md`, AgX).

| Name | darktable field | Type | Range | Default | Meaning |
| --- | --- | --- | --- | --- | --- |
| Look offset | `look_lift` | f32 | −1 to 1 | 0 | Shifts the curve's output: `(slope / (1 + lift)) · (y + lift)`, so at slope 1 it lifts or deepens shadows and keeps 1 fixed |
| Look slope | `look_slope` | f32 | 0 to 10 | 1 | Multiplies the curve's output |
| Look power | `look_brightness` | f32 | 0 to 100 | 1 | darktable's "brightness" `b`, applied as the power `1/b` for `b ≥ 1` and `1/√b` below, to positive values |
| Look saturation | `look_saturation` | f32 | 0 to 10 | 1 | Scales each channel's distance from the rendering space's luminance |
| Preserve hue | `look_original_hue_mix_ratio` | f32 | 0 to 1 | 0.6 | 0 keeps the curve's hue shifts, 1 restores the input's HSV hue |
| Black relative exposure | `range_black_relative_ev` | f32 | −20 to −0.1 | −10 | EV below mid-grey that maps to the curve's start |
| White relative exposure | `range_white_relative_ev` | f32 | 0.1 to 20 | 6.5 | EV above mid-grey that maps to the curve's end |
| Dynamic range scaling | `dynamic_range_scaling` | f32 | −0.5 to 2 | 0.1 | Safety margin the pickers add to the range |
| Pivot relative exposure | `curve_pivot_x` | f32 | 0 to 1 | 10/16.5, i.e. 0 EV | The pivot's input, as a fraction of the range |
| Pivot target output | `curve_pivot_y_linear_output` | f32 | 0 to 1 | 0.18 | The pivot's linear output |
| Contrast | `curve_contrast_around_pivot` | f32 | 0.1 to 10 | 3.0 | Slope of the linear section, normalised to a 16.5 EV range and gamma 2.2 |
| Toe start | `curve_linear_ratio_below_pivot` | f32 | 0 to 1 | 0 | Share of the output range below the pivot kept linear |
| Shoulder start | `curve_linear_ratio_above_pivot` | f32 | 0 to 1 | 0 | Share of the output range above the pivot kept linear |
| Toe power | `curve_toe_power` | f32 | 0 to 10 | 1.5 | Toe hardness; at least 0.01 in use |
| Shoulder power | `curve_shoulder_power` | f32 | 0 to 10 | 3.3 | Shoulder hardness; at least 0.01 in use |
| Curve y gamma | `curve_gamma` | f32 | 0.01 to 100 | 2.2 | The curve shapes `output^(1/γ)`; the slope is compensated so contrast at the pivot stays |
| Auto gamma | `auto_gamma` | bool | | off | Sets γ so the pivot lies on the diagonal |
| Target black | `curve_target_display_black_ratio` | f32 | 0 to 0.15 | 0 | Linear output at the curve's start |
| Target white | `curve_target_display_white_ratio` | f32 | 0.2 to 1 | 1 | Linear output at the curve's end |
| Base primaries | `base_primaries` | enum | 0 export profile, 1 working profile, 2 Rec2020, 3 Display P3, 4 Adobe RGB (compatible), 5 sRGB | 2 | The space whose primaries are inset and rotated |
| Disable adjustments | `disable_primaries_adjustments` | bool | | off | Zeroes every inset, rotation, outset and unrotation: the curve alone |
| Attenuation (inset) | `red_inset`, `green_inset`, `blue_inset` | f32 | 0 to 0.99 | 0.29462451, 0.25861925, 0.14641371 [0] | Moves each primary towards white before the curve, so that colour desaturates sooner in the highlights |
| Rotation | `red_rotation`, `green_rotation`, `blue_rotation` | f32, radians | ±0.5236 | 0.03540329, −0.02108586, −0.06305724 [0] | Rotates each primary about white before the curve |
| Master purity boost | `master_outset_ratio` | f32 | 0 to 2 | 1 | Scales every outset |
| Master rotation reversal | `master_unrotation_ratio` | f32 | 0 to 2 | 0 [1] | Scales every unrotation |
| Purity boost (outset) | `red_outset`, `green_outset`, `blue_outset` | f32 | 0 to 0.99 | 0.290776401758, 0.263155400753, 0.045810721815 [0] | Moves each primary of the matrix inverted after the curve towards white, which restores purity |
| Reverse rotation | `red_unrotation`, `green_unrotation`, `blue_unrotation` | f32, radians | ±0.5236 | equal to the rotations [0] | After the curve, undoes the rotation |
| Reverse all | `completely_reverse_primaries` | bool | | off | Outsets equal the insets, unrotations the rotations, both master ratios 1 |

## Parameter versions

| Version | Change | Migration |
| --- | --- | --- |
| 1 to 6 | Unreleased test versions from AgX's development | `legacy_params` discards them and returns the scene-referred defaults as version 7, whatever they held; so does `ports.rs`. Verified: test 0174's version 4 entry renders identically (ΔE2000 0) to the same sidecar with explicit version 7 defaults |
| 7 | Current in the reference release, and the first released (5.4.0) | |
| 8 | Not in the reference release: master (c10bf49424) appends `target_white_from_output` (i32) and `hdr_purity` (f32), 152 bytes | Master's 7 → 8 copies the fields, sets following off and purity 0.5. darktable cannot read version 8: it substitutes the module's default params, without a message in darktable-cli. Apertrust refuses version 8 |

## Kernel shape

The three parts of decision 8:

1. **The parameter struct** `AgxParams`, 288 bytes after the 48-byte pass header, in 16-byte steps, written by `agx::Prepared::params`: the matrices `pipe_to_base`, `base_to_rendering` and `rendering_to_pipe` by rows, the Y row of `rendering_to_xyz`, the log encoding and linear section (black EV, range, γ, slope, intercept, target black and white), the toe and the shoulder (transition, scale, power, fallback coefficient, power and switch), and the look (slope, lift, power, saturation, hue mix). The derivative entry points differentiate against `AgxLook : IDifferentiable`, built from the look's fields.
2. **Once-per-image setup on the CPU**, `agx::prepare`: `Matrices::new` (the primaries), `Curve::new` (the coefficients) and `Look::new`, each a pure function of the settings. No tables. `Curve::encode` and `Curve::eval` evaluate the curve as the kernel does, for tests and a curve graph.
3. **A pure per-pixel function**, `[Differentiable] float3 agx(no_diff float3 rgb, no_diff AgxParams p, AgxLook look)`. Everything before the look depends only on the pixel and constants and sits in the plain `agx_tone_curve`, built on the pure `agx_curve(x, p)`; the look, the linearisation, the hue restoration and the output matrix are differentiable. `main`, `agx_backward` and `agx_forward_diff` only wrap it.

Shared code it uses: `RgbSpace` (the lcms-built RGB spaces darktable offers as base primaries, with their D50 chromaticities), `rotate_and_scale_primary`, `rgb_to_xyz_from_chromaticities` and `checked_inverse_d` (`mat3SSEinv`'s refusal of a singular matrix) in `apertrust-color`'s `src/lib.rs`; `rgb_to_hsv` and `hsv_to_rgb` in its `slang/color.slang`.

## Setup on the CPU (`commit_params`)

`ε = 1e-6` throughout. The curve is computed in f32, as darktable does; the matrices in f64.

**Look.** Lift, slope and saturation as given; `power = b < 1 ? 1/√max(b, ε) : 1/b` from `look_brightness` b; the hue mix as given. darktable also sets `look_tuned` (slope ≠ 1, brightness ≠ 1, lift ≠ 0 or saturation ≠ 1) and `restore_hue` (mix ≠ 0); the port computes both steps unconditionally (Deliberate differences).

**Curve** (`_calculate_tone_mapping_params`, `_adjust_pivot`, `_calculate_slope_gamma_compensation`):

- `range = white_ev − black_ev`; `pivot_x = clamp(curve_pivot_x, ε, 1 − ε)`.
- `γ = curve_gamma`, or with auto gamma `log2(pivot_y_lin) / log2(pivot_x)` when both are positive, clamped to [0.01, 100].
- `y_lin = clamp(pivot_y_lin, black_ratio, white_ratio)`; `pivot_y = y_lin^(1/γ)`.
- Slope: `contrast · range / 16.5`, divided by `γ · max(ε, pivot_y)^(γ−1) / (2.2 · max(ε, y_lin^(1/2.2))^1.2)`, which keeps the linearised slope at the pivot what it is at γ = 2.2.
- Toe: `tb = black_ratio^(1/γ)`, `toe_p = max(0.01, toe_power)`. `toe_tx = max(ε, pivot_x − (pivot_y − tb) · toe_start / slope)`, `toe_ty = pivot_y − slope · (pivot_x − toe_tx)`. `toe_scale = −scale(1, 1 − tb, 1 − toe_tx, 1 − toe_ty, slope, toe_p)`, the shoulder's formula on the flipped toe. With `dy = max(ε, toe_ty − tb)`: the convex fallback is used when `dy / toe_tx > slope`, with power `slope · toe_tx / dy` and coefficient `dy / toe_tx^power`.
- `intercept = toe_ty − slope · toe_tx`.
- Shoulder: `tw = white_ratio^(1/γ)`, `sh_p = max(0.01, shoulder_power)`. `sh_tx = min(1 − ε, pivot_x + (tw − pivot_y) · shoulder_start / slope)`, `sh_ty = pivot_y + slope · (sh_tx − pivot_x)`, `sh_scale = scale(1, tw, sh_tx, sh_ty, slope, sh_p)`. With `dy = max(ε, tw − sh_ty)` and `dx = 1 − sh_tx`: the concave fallback is used when `dy / dx > slope`, with power `slope · dx / dy` and coefficient `dy / dx^power`.
- `scale(lx, ly, tx, ty, m, p) = min(1e9, max(ε, a^−p − r^−p)^(−1/p))`, with `a = max(ε, ly − ty)` and `r = m · max(ε, lx − tx)` (`_scale`): the sigmoid's scale that makes it reach (lx, ly) from the transition at slope m. Computed in the log domain (Precision).

At the defaults: γ 2.2, slope 3, pivot (0.6061, 0.4587), no linear section, no fallback. Test 0175's curve: γ 2.3, slope 2.686, pivot (0.6529, 0.5177), short linear sections, no fallback. `agx-edges`: slope 0.82, both fallbacks.

**Primaries** (`_get_primaries_params`, `_create_matrices`). Matrices are column-vector, `M · v`; darktable stores them transposed.

- With disable adjustments, every inset, rotation, outset and unrotation is 0. Otherwise, with reverse all, outsets equal the insets, unrotations the rotations, and both master ratios are 1.
- The base profile: Rec2020, Display P3, Adobe RGB (compatible) or sRGB as darktable builds them in lcms (`colorspaces.c`: those primaries and white (0.31271, 0.32902)), or the working profile. Only a profile's matrices and chromaticities are used, never its transfer curve. Its chromaticities are what darktable reads back from the profile (`dt_colorspaces_get_primaries_and_whitepoint_from_profile`): the xy of its colorant tags, which lcms Bradford-adapts to D50, and of its media white, which lcms writes as D50. So the insets and rotations work in D50 chromaticities, which is how darktable's Blender-like defaults reproduce Blender's matrices. darktable's `dt_D50_XYZ_to_xyY` clamps negative XYZ to 0 first, and the adapted red colorants of Rec2020 and Display P3 (z = 0 in D65) have a Z of about −0.002: their red primary moves slightly, and the port clamps the same way.
- `pipe_to_base = base_out · work_in`, exactly the identity when the base profile is the working profile (darktable then skips the product); `base_to_pipe` its inverse.
- Inset primary i: `rotate_and_scale(base, 1 − inset_i, rotation_i)`. `rendering_to_xyz` is the matrix of the inset primaries and the base white; `base_to_rendering = base_out · rendering_to_xyz`.
- Outset primary i: `rotate_and_scale(base, 1 − master_outset · outset_i, master_unrotation · unrotation_i)`, `outset_to_xyz` its matrix; `rendering_to_pipe = base_to_pipe · (base_out · outset_to_xyz)^−1`, or 0 where `mat3SSEinv` refuses that inverse (Precision).
- `rotate_and_scale(profile, s, θ, i)` (`dt_rotate_and_scale_primary`): the direction from the white to primary i, rotated by θ; the distance from the white along it to the nearest edge of the primaries' triangle; the new primary is the white plus s times that distance along it.
- Matrix from primaries and white (`dt_make_transposed_matrices_from_primaries_and_whitepoint`, Lindbloom): columns `(x/y, 1, (1 − x − y)/y)` per primary, with `y` kept at least `FLT_EPSILON` from zero, each scaled so the columns sum to the white's XYZ.

With zero insets, rotations and outsets, the way out undoes the way in. `base_to_rendering` is then the identity for sRGB and Adobe RGB, but not quite for Rec2020 and Display P3 (up to 0.0024), because of the clamped red primary: in darktable too, the curve then runs in a space very slightly different from the base.

## Per pixel (`process`)

1. Clamp each channel to [−1e6, 1e6] (`process`, about 22.5 EV above mid-grey). Alpha passes through.
2. `base = pipe_to_base · rgb`.
3. `_compress_into_gamut`, with luminance weights `L = (0.2658180370250449, 0.59846986045365, 0.1357121025213052)` whatever the base primaries: `Y_c = max(M − rgb) − L·(M − rgb) + L·rgb` with `M = max(rgb)`; `o = max(−min(rgb), 0)`, `rgb' = rgb + o`; `Y_n` the same expression for `rgb'`; `rgb = rgb' · (Y_n > Y_c and Y_n > ε ? Y_c / Y_n : 1)`. As the weights sum to 1, this is `rgb' · Y_c / (Y_c + o)` where `Y_c + o > ε`, with `Y_c = 2·L·rgb − min(rgb)`: only pixels with a negative channel change. The port keeps darktable's form.
4. `r = base_to_rendering · base`.
5. `_agx_tone_mapping`:
   1. `h_in`, the HSV hue of `r` (`dt_RGB_2_HSV`: zero when `max ≤ 1e-6` or `max − min ≤ 1e-6`).
   2. Per channel: `x = clamp((log2(max(ε, r/0.18)) − black_ev) / range, 0, 1)` (`_apply_log_encoding`), then `y = curve(x)`.
   3. The look (`_agx_look`): `v = (slope / (1 + lift)) · y + lift · slope / (1 + lift)`; `v = v > 0 ? v^power : v`; `ℓ = rendering_luminance · v`; `v = ℓ + saturation · (v − ℓ)`.
   4. Linearise: `v = max(v, 0)^γ`.
   5. Restore hue: `(h, s, value)` the HSV of `v`; `h = wrap(h_in + (1 − mix) · d)`, with `d = (h − h_in)` reduced to [−0.5, 0.5] and `wrap(t) = t − floor(t)` (`_lerp_hue`); `v` from `(h, s, value)` (`dt_HSV_2_RGB`).
6. `out = rendering_to_pipe · v`.

The curve (`_apply_curve`), for `x` in [0, 1], with `σ(t, p) = t / (1 + t^p)^(1/p)`:

- `x < toe_tx`: convex fallback `tb + max(0, coefficient · x^power)`, or `toe_scale · σ(slope · (x − toe_tx) / toe_scale, toe_p) + toe_ty`;
- `x ≤ sh_tx`: `slope · x + intercept`;
- otherwise: concave fallback `tw − max(0, coefficient · (1 − x)^power)`, or `sh_scale · σ(slope · (x − sh_tx) / sh_scale, sh_p) + sh_ty`;
- then clamped to [tb, tw].

The sigmoid's argument is never negative: in the toe both `x − toe_tx` and `toe_scale` are negative. The pieces join with equal slope (σ has slope 1 at 0, and the fallbacks' powers match the slope), and the scales make the toe end at (0, target black) and the shoulder at (1, target white).

## Neighbourhood and scale

- **Neighbourhood radius:** none; a per-pixel module.
- **Scale dependence:** none.
- **Tile overlap needed:** none.
- **Region-of-interest changes:** none (no `modify_roi_in` or `modify_roi_out`).

## Precision and special values

- **fp64:** darktable's processing path is fp32 throughout. Its doubles are in the GUI only: the curve graph's cairo calls, and `0.8 * exposure` in `_adjust_relative_exposure_from_exposure_params` ("read exposure"). lcms builds the profile matrices in double, and the port builds every matrix in f64 on the CPU (as colorbalancergb does). After the tag, master moved `_scale` to double (a7053e1369), because `powf(ε, −p)` overflows to infinity for powers above about 6.4: with pivot target output at target white the scale became zero and the curve NaN, so everything above the pivot rendered black. The port computes `_scale` in the log domain instead, `log2 base = −p·log2 a + log2(1 − 2^(−p·(log2 r − log2 a)))` when `r > a`, else `log2 ε`, clamped below at `log2 ε`, then `scale = min(1e9, 2^(−log2 base / p))`. That is finite in fp32 for every slider value, equal to darktable's within 1e-5 wherever darktable's is finite (unit test), and stays fp32 so it could move into a differentiable Slang setup later.
- **The sigmoid:** `t^p` overflows fp32 for large arguments, which only extreme settings reach (master caps the shoulder power at 10 for HDR for this reason). The kernel uses `t · (1 + t^p)^(−1/p)` for `t ≤ 1` and the equal `(1 + t^−p)^(−1/p)` above.
- **NaN:** darktable maps NaN input to 0 (`isnan`). The kernel cannot rely on NaN checks; Apertrust sanitises at operation boundaries, and the kernel keeps only the ±1e6 clamp.
- **Logarithms and divisions:** `log2` sees at least ε; the slope, the range (at least 0.2 EV), the transitions' x distances (at least ε) and the luminance ratio (`Y_n > ε`) are all guarded in darktable's own maths.
- **Auto gamma** is unbounded in the reference release: pivot target output 1 gives γ = 0 and an infinite 1/γ, and a pivot near 1 a huge γ. Master clamps it to [0.01, 100] (3526ecc350), and so does the port; only golden cases can turn it on, since no module exposes it.
- **Hue rounding:** `t − floor(t)` rounds to exactly 1.0 for `t` within about 3e-8 below 0. darktable's `dt_HSV_2_RGB` then falls into its last sector with f = 0 and renders red as magenta. The port treats hue 1 as hue 0 (Deliberate differences).
- **A singular outbound matrix.** The way out inverts `base_out · outset_to_xyz` with `mat3SSEinv`, and that matrix is singular wherever the white lies on the line through two outbound primaries, so it is no mix of them with the third. Two surfaces of the settings reach it, inside the sliders' ranges:
  - *An outbound primary on the white*: a master purity boost times a purity boost of 1 makes that primary's scaling 0, reached by any master purity boost from 1/0.99 up (200% with a boost of 50%, 125% with 80%). One primary there leaves the white no share of the other two, and two make the primaries' own matrix singular, which `rgb_to_xyz_from_chromaticities` turns to 0.
  - *Two outbound primaries opposite each other across the white*: reverse rotations turning two primaries 180° apart. A master rotation reversal of 1 reaches it, so reverse all does too: for example green turned −27.6° and blue +27.6° over Rec2020, ∓22.0° over Display P3, ∓25.8° over Adobe RGB and ∓19.4° over sRGB (measured with a search along such pairs).

  **What darktable does there.** Where the determinant is under 1e-7 in magnitude, darktable's `mat3SSEinv` refuses it and returns without writing. `_create_matrices` declares `rendering_to_base_transposed` without initialising it, so darktable renders whatever its stack held. Where the f32 rounding of a singular matrix escapes the refusal instead, darktable inverts that rounding.

  darktable-cli (macOS, arm64), on `agx-edges`'s sidecar (sRGB base) with reverse all off, rendered every pixel between 3.9e22 and 1.1e29 at master 200% with red and green boosts at 50%, with red alone at 50%, and at master 125% with red at 80%. That is no result to follow: it is an uninitialised read, or rounding inverted. At 125% and 80% even whether the scaling is 0 (`1 − 1.25 · 0.8f` rounds to 0) or −1.5e-8 depends on whether the build fuses the multiply and add.

  **What the port does.** The setup takes the decision `mat3SSEinv` is written to make, with the determinant in f64 (`checked_inverse_d` in `apertrust-color`), and uses the zero matrix there: the image is black (Deliberate differences). The refusal is narrow:

  - at master 200% over sRGB, a boost within about 4e-5 of 50%, which of the slider's values (two decimals of a percent) is 50.00% alone;
  - at the rotations' surface over Rec2020, about one f32 step of a rotation (6e-8 rad).

  Off it, the matrix is darktable's, its entries growing as one over the distance: 1.2e3 at a boost of 49.99%, 9.7e6 one f32 step from the rotations' surface. darktable computes it in f32, so near the surfaces its rounding is magnified as much, which the port's f64 matrices leave out. At master 200% with a red boost of 49.9% (scaling 0.002, sRGB base) the port differs from darktable-cli's render by ΔE2000 mean 0.0011, max 0.047, and at 49% (scaling 0.02) by mean 0.0002, max 0.0067 (ad hoc renders of the same sidecar, not cases).

  Over every corner of the outbound settings (each master at 0, 1 and 2, each boost at 0, 0.5 and 0.99, each reverse rotation at ±30° and 0, over the four base primaries) and of the inbound ones under reverse all, every matrix is finite, black exactly where master 200% meets a boost of 50%, and its entries below 1e4 elsewhere (unit tests).

  The inbound matrix is never inverted: where the white lies on the line through two inbound primaries (the same rotations under reverse all) the rendering space loses a dimension, finite, as in darktable.

## CPU and OpenCL differences in darktable

None that change results. `agx.cl` reduces the hue difference with `x − rint(x)` instead of `remainderf(x, 1)` (the same value), uses `dtcl_pow` for `powf`, and maps NaN with `select(…, isnan)`. The references are rendered on the CPU (`--disable-opencl`).

## Deliberate differences

- **The look and the hue restoration always run.** darktable skips them at their neutral values (`look_tuned`, `restore_hue`), where they are identities up to rounding (within 1e-6 for near-grey pixels, which HSV makes exactly grey). Branching would make the derivatives at the neutral values zero. `agx-default`, `agx-highlights` (neutral look) and `agx-edges` (hue mix 0) cover darktable's skipped paths at ΔE2000 max 0.0005.
- **Hue 1 is hue 0** in `hsv_to_rgb`: where darktable's wrap rounds a tiny negative hue to 1.0 and renders a red as magenta, the port renders the red. darktable reaches it only within about 3e-8 of the wrap; no pixel of the four golden cases hits it in darktable's renders, so their ΔE is the same either way. Where it happens, at reds whose green and blue the curve clips to the same black, darktable's output jumps by 0.2.
- **The look's `v > 0 ? v^power : v`** is written `pow0(max(v, 0), power) + min(v, 0)`, the same values without a branch; the look's and the linearisation's powers use `pow0` (Slang's `pow` derivative trap). HSV computes every candidate hue and selects one, so no value the backward pass needs exists only inside a branch.
- **The fallbacks' `x < 0` and `x ≥ 1` branches go:** after the clip, `x` is never negative, and at `x = 1` both sides of the shoulder's branch give target white.
- **`_scale` in the log domain and the overflow-free sigmoid** (Precision): finite where the reference release renders NaN.
- **Auto gamma clamped to [0.01, 100]**, following master.
- **Matrices computed in f64 and rounded to f32 once**; darktable works in f32, including `mat3SSEinv` and the custom primaries.
- **Singular outbound settings render black, where darktable reads an uninitialised matrix** (Precision): an outbound primary on the white (a master purity boost times a purity boost of 1) or the white on the line through two outbound primaries (reverse rotations turning two of them 180° apart), within `mat3SSEinv`'s refusal of a determinant under 1e-7.

  darktable leaves `rendering_to_base_transposed`, never initialised, as the stack held it, or inverts f32 rounding where that escapes the refusal: values of 3.9e22 to 1.1e29 at every pixel on the build measured, reproducible on no build.

  The setup refuses with the determinant in f64 and uses the zero matrix, as darktable's `dt_make_transposed_matrices_from_primaries_and_whitepoint` does with its zeroed inverse and the rgb primaries port does for its singular settings (`docs/ports/primaries.md`). The size: black against darktable's undefined values, at master 200% for a purity boost of 50.00% alone of the slider's values. The output is constant there, so its derivatives are 0.
- **The hue difference uses `x − round(x)`**, as darktable's OpenCL does; WGSL's `round` rounds half to even, like `remainderf`.
- **No NaN mapping**, as above.
- **Base primaries "export profile" returns `NotPorted`**: the image context carries the output profile (decision 26), but AgX does not read it yet. Once it does, its primaries come from `ImageContext::output_profile`, colorout's `type` as the edit sets it, as darktable's `dt_ioppr_get_export_profile_type` reads colorout's `type` from the edit, the working profile when there is none (`None`), and a profile without a matrix falling back to Rec2020 as darktable's does. darktable takes the edit's profile even when an export overrides it; when Apertrust's export gains its own profile, this reads the edit's, a field of its own then (decision 26). Its golden case, `agx-export-profile`, renders with an sRGB output profile, compared after colorout. "Working profile" is Rec2020, Apertrust's fixed working space.
- **Not ported (GUI only):** dynamic range scaling, the pickers, "read exposure", the curve graph and its warnings.
- **Params version 8 is refused**, where darktable silently substitutes defaults.

## Tool binding notes

- **In the first pass** (decision 2) the AgX module is darktable's, built ([../modules/agx.md](../modules/agx.md)): its controls are the parameters above under darktable's labels, the pivot stored in EV as darktable's slider shows it, dynamic range scaling a control not ported, and darktable's looks are its presets in the header's presets menu, each setting the range too, as darktable's do. AgX is one of the tone mappers, modules of their own, one per edit; sigmoid and filmic rgb join as they are ported.
- The port renders whatever a sidecar sets: test 0175 carries pivot target output 0.22, gamma 2.3 and target black and white 0.0015 and 0.9485.
- A purity boost of 50% at a master purity boost of 200% (any product of 1), or reverse rotations turning two primaries opposite each other, blacks the image out (Precision; darktable shows noise there), and close to either the colours are boosted without bound. The defaults (master purity boost 100%, master rotation reversal 0) are far from both: a purity boost reaches the first only with the master purity boost above 101%, and the second needs green's and blue's reverse rotations, times the master rotation reversal, turned about 39° (sRGB) to 55° (Rec2020) further apart, so a master rotation reversal of at least about 65% to 92%, or reverse all with green's and blue's rotations that far apart.

## Gradients

Parameters, in the derivative entry points' order: `look_slope`, `look_power` (the kernel's power, which the finite differences move directly, not darktable's brightness), `look_saturation` and `look_original_hue_mix_ratio`. With the curve's coefficients set up on the CPU, these enter only the per-pixel function, so the kernel's derivatives are their total derivatives. `look_lift` is differentiated but not returned: it shares the slope's affine path, and the interface takes four. The check covers the look, the linearisation, the HSV round trip with the hue mix, and the output matrix; the curve and the gamut compression come before every differentiated parameter.

Results, Apple M4, step 1e-3, relative error of reverse mode against central finite differences, and forward against reverse mode per pixel (largest over the four parameters):

| Case | look_slope | look_power | look_saturation | hue_mix | Forward vs reverse |
| --- | --- | --- | --- | --- | --- |
| `agx` | 4.9e-5 | 4.7e-5 | 4.5e-5 | 1.3e-5 | 1.9e-7 |
| `agx-default` | 1.7e-5 | 1.7e-5 | 1.8e-5 | 6.5e-6 | 2.6e-7 |
| `agx-highlights` | 1.7e-5 | 1.7e-5 | 1.7e-5 | 1.4e-5 | 2.4e-7 |
| `agx-edges` | 1.9e-5 | 1.3e-5 | 1.3e-5 | 3.9e-6 | 5.0e-7 |
| synthetic, `agx` params | 5.1e-5 | 4.5e-5 | 5.0e-5 | 2.0e-5 | 1.3e-7 |
| synthetic, defaults (`agx-default`, `agx-highlights`) | 1.7e-5 | 2.1e-5 | 7.4e-6 | 1.8e-5 | 2.1e-7 |
| synthetic, `agx-edges` params | 1.4e-5 | 8.1e-5 | 1.2e-5 | 4.0e-4 | 3.3e-7 |

**The hue mix at 0 sits on HSV's corners.** `agx-edges` has hue mix 0, darktable's skipped-restoration path. When the curve clips two channels to the same value (both above white, or both below black), the processed hue is exactly a sector boundary of HSV: 1/6, 1/2 or 0 (on the worst pixels, bright oranges come out with hue exactly 1/6, bright blues and cyans exactly 1/2). HSV to RGB is piecewise linear in the hue with a corner at each sector boundary, and at mix 0 restoring the hue leaves it exactly there. Slang's own derivative takes the slope of the sector on the right, while central differences average both sides: a disagreement of 1.8e-2 on the case and 6.7e-2 on the sweep, at every step from 1e-4 to 1e-2.

`hsv_to_rgb` in `apertrust-color`'s `slang/color.slang` therefore writes out its derivatives: within 1e-5 sectors of a boundary they average the slopes of the sectors either side, the derivative's natural value at a corner and what central differences measure. Only the derivatives are written out; the forward pass is plain HSV arithmetic. An optimiser at mix 0 sees the average of the two slopes rather than one of them.

Other non-smooth spots, where finite differences straddle a kink: the log encoding's clip at the range's ends (the curve is flat outside), the curve's clamp to [target black, target white], the look's kink at 0 for a power other than 1, the linearisation's clip at 0, HSV's choice of the largest channel, HSV's zero hue for near-greys (`max − min ≤ 1e-6`), and the hue difference's wrap at ±0.5.

**Planned with reference matching in Phase 3:**

- **The range and the curve.** Black and white exposure, toe and shoulder power and start act through the coefficients the CPU sets up, so the kernel cannot differentiate them yet. The curve's setup becomes a `[Differentiable]` Slang function evaluated in the kernel (the log-domain `_scale` keeps it fp32-safe), so the curve controls' derivatives come from autodiff, not from a Jacobian of the CPU setup.
- **The input.** Every upstream module's gradient reaches the output through AgX, so end-to-end optimisation needs AgX's derivative with respect to its input pixel. The derivative interface gains input-derivative entry points, per-pixel operations first, AgX among them; for a per-pixel operation reverse mode needs no scatter, and `agx_tone_curve` becomes differentiable. Spatial operations record their gap as understood (decision 11).

## Test plan

- **Golden cases** (`crates/golden/src/cases.rs`), all with `disable: &[]` (the sidecars have no later tone mapper; AgX, the module under test, stays on in the expected render) and no mask:
  - `agx`, test 0175, mire1.cr2: every look control off its neutral value, hue mix 0.6157, Adobe RGB base primaries with custom insets, rotations, outsets and unrotations (master rotation reversal 0), toe and shoulder starts (a linear section), target black and white, γ 2.3, pivot target output 0.22, range −9.12 to +4.85 EV. The sidecar holds two AgX entries; the second is active.
  - `agx-default`, test 0174, mire1.cr2: params version 4, which the reference release migrates to its scene-referred defaults, so it covers that migration.
  - `agx-highlights`, test 0186 (agx-sdr-legacy), hlrecovery.arw: the version 7 defaults on a raw with clipped highlights: the shoulder, the log encoding's top clip, and bright saturated colours.
  - `agx-edges`, test 0174's sidecar edited (its `Case` comment says how): version 7 defaults with look lift 0.05, slope 0.95, brightness 1.2 (power 1/b), saturation 0.85, hue mix 0, range −2 to +2.5 EV with the pivot at mid-grey (both fallback curves), sRGB base primaries and reverse all. Auto gamma stays off.
  - Display P3 base primaries are not in a case yet: one AgX instance has one base. An ad hoc check, `agx-edges` with Display P3 rendered by darktable-cli and by the kernel outside the harness, gave ΔE2000 mean 0.0000, max 0.0007 (module effect mean 6.48). Planned: a fifth case, `agx-edges` with Display P3 base primaries.
  - Not cases: 0188 (agx-hlg-legacy) differs from 0186 only in output colour's profile (HLG Rec2020), which the harness replaces with linear Rec2020 (`--icc-type LIN_REC2020`), and AgX in the reference release does not depend on the output. 0187 and 0189 (sdr-follow, hlg-follow) carry params version 8, which the reference release replaces with its defaults. All three render identically to 0186 (ΔE2000 0, measured); they become cases when the reference release has version 8 and the harness renders HDR output.
- **Tolerance:** `Tolerance::DARKTABLE` (mean ΔE2000 ≤ 0.767, max ≤ 2.3) for every case.
- **Invariants:**
  - On the GPU (`crates/golden/tests/agx.rs`, run in CI): with the pivot at mid-grey and a neutral look, grey 0.18 renders to 0.18 within 2e-4 for Rec2020, Adobe RGB, sRGB and Display P3 bases; output is finite for 0, ±1e6, 1e-30, negative and very saturated input, at the defaults, the narrow range, pivot target output 1 with toe and shoulder power 10 (where the reference release renders NaN), the sliders' extremes, and two outbound primaries on the white (master purity boost 2, red and green boosts 0.5).
  - On the CPU (`agx.rs`, `color.rs`): the curve is non-decreasing on [0, 1], starts at target black, ends at target white, passes through the pivot and joins its pieces with equal slope; mid-grey encodes to the pivot; the narrow range needs both fallbacks; the log-domain `_scale` matches darktable's where finite and stays finite where it is not; every matrix keeps white; unmodified primaries cancel; an outbound primary on the white (master purity boost 2 with boosts of 0.5, 1.25 with 0.8) gives the zero outbound matrix over every base; every corner of the outbound settings, and of the inbound ones under reverse all, gives finite matrices, zero exactly where a primary is on the white and with entries below 1e4 elsewhere; `checked_inverse_d` refuses a determinant under 1e-7; brightness maps to power; "export profile" is `NotPorted`; the D50 chromaticities rebuild lcms's matrices, with darktable's clamp of negative colorants.
  - In `ports.rs`: test 0186's stored params equal `Settings::scene_referred_default` field for field, version 4 gives the defaults, version 8 is refused.

## Changes after the reference release

Fifteen commits change `src/iop/agx.c` or `data/kernels/agx.cl` after the reference release, up to master at 06a2ba46e7 (2026-10-28) (`git log $DT_TAG..upstream/master -- src/iop/agx.c data/kernels/agx.cl`). Four of them come from the output-headroom merge (2af685755d, 2026-10-21), whose HDR tests darktable's integration tests (commit 3f2d1ab) already have. By effect:

- **Behaviour, to evaluate when the reference moves:** c10bf49424 renders for the output's peak, as Blender 5's AgX HDR (params version 8: following the peak raises the shoulder power by `H^log10(2)`, adds a second log2-space sigmoid after linearising, and an "HDR purity" control; SDR unchanged), with e5d050c91e (refactor) and 84532d8eae (the pivot picker uses the SDR curve). Relevant when Apertrust gets system-managed HDR display (Phase 7), and to target white, which becomes relative to the output's peak.
- **Bug fixes the port already follows:** a7053e1369 (`_scale` in double) and 3526ecc350 (auto gamma clamped to [0.01, 100], and recomputed by "read exposure").
- **No effect on output:** 5cfca8aecf (tiling allowed), b299a9df0f, 57c0592ff0, eac6d9130d, fd9b0d61d3, 5f3806ec6e, 06e85df41b, c2dd84a409 (GUI), a42879ae4f and 7cb4369080 (style).
