# Port spec: filmic rgb

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/filmicrgb.c` `legacy_params()`, `convert_to_spline_v3()`, `dt_iop_filmic_rgb_compute_spline()`, `commit_params()`, `reload_defaults()` and `process()`, with helpers from `src/common/bspline.h` (`blur_2D_Bspline`), `src/develop/noise_generator.h` (`splitmix32`, `xoshiro128plus`, `dt_noise_generator_simd`), `src/iop/gaussian_elimination.h` (`gauss_solve`), `src/common/gamut_mapping.h` (`RGB_to_Ych`, `Ych_to_RGB`, `prepare_RGB_Yrg_matrices`, `Ych_max_chroma`), `src/common/colorspaces_inline_conversions.h` (Yrg and Ych, `gamut_check_Yrg`), `src/common/iop_profile.h` (`dt_ioppr_get_rgb_matrix_luminance`) and `src/common/math.h` (`dt_vector_powf`, its `dt_vector_log2` and `dt_vector_exp2`). OpenCL: `data/kernels/filmic.cl` and `data/kernels/bspline.cl`. The reference is darktable's CPU path.

Status: ported.

- **Code:** the kernel is `slang/filmicrgb.slang` in `crates/op/filmicrgb`, its setup `src/setup.rs`, its manifest and schema `src/lib.rs`.
- **Coverage:** everything darktable offers a new instance: the five colour sciences, v3 (2019) to v7 (2023), darktable's default; every norm, the legacy euclidean one included; the hard, soft and safe curves of the toe and the shoulder; the display targets, custom middle grey; and the highlight reconstruction, its noise in all three distributions, its wavelets in RGB and its iterations on ratios. Only spline v1 and v2 return `NotPorted`: no widget sets them, and only a params version 5 edit holds them (Parameter versions).
- **Planning:** in `OPERATIONS` at darktable's v5.0 place, working space 46.0, with `default_enabled` off: nothing plans it until an edit can choose it as its tone mapper (decision 33, planned with the Filmic RGB module).
- **Checks:** the eight golden cases pass, and the gradient check passes on every case and on the synthetic sweep (Gradients). Checked on one GPU so far.

Golden results, Apple M4, each case a one-instance plan of the engine (decision 15):

| Case | darktable test | Settings | ΔE2000 mean | p95 | max | Module effect (mean) | Render (median, min) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `filmicrgb` | 0014-filmic-rgb | version 1: v3 (2019), power norm, hard toe and soft shoulder, custom grey 9.225 %, reconstruction past every pixel | 0.0000 | 0.0001 | 0.0001 | 11.8 | 297 ms, 282 (37 passes) |
| `filmicrgb-reconstruction` | 0037-filmic-reconstruction | version 4: v4 (2020), power norm, soft toe and hard shoulder, reconstruction at +0.5 EV, one iteration, no noise | 0.0001 | 0.0002 | 0.0054 | 4.52 | 607 ms, 556 (72 passes) |
| `filmicrgb-v6` | 0092-filmicrgb-v6 | version 5: v6 (2022), no norm, safe curves, reconstruction at +3 EV, gaussian noise | 0.0000 | 0.0001 | 0.0003 | 3.88 | 591 ms, 546 (72) |
| `filmicrgb-v7` | 0138-filmic-v7 | v7 (2023), safe curves, reconstruction at +3 EV, poissonian noise | 0.0001 | 0.0001 | 0.0006 | 4.54 | 616 ms, 546 (72) |
| `filmicrgb-v3-nochroma` | 0139, its second entry | v3 (2019), no norm, safe toe and soft shoulder, custom grey | 0.0001 | 0.0003 | 0.0292 | 13.0 | 302 ms, 281 (37) |
| `filmicrgb-v4-nochroma` | 0140, its second entry | v4 (2020), no norm, safe toe and hard shoulder, reconstruction at +3 EV, uniform noise | 0.0000 | 0.0001 | 0.0087 | 3.05 | 616 ms, 547 (72) |
| `filmicrgb-v5-nochroma` | 0141, its third entry | v5 (2021), no norm, safe curves, no reconstruction | 0.0000 | 0.0001 | 0.0004 | 4.06 | 4.5 ms (1 pass) |
| `filmicrgb-v6-euclidean` | 0142 | v6 (2022), euclidean norm, safe curves, reconstruction at +3 EV | 0.0001 | 0.0001 | 0.0004 | 2.91 | 629 ms, 567 (72) |

No pixel of any case is over 2.3. The module's effect is darktable's input against its output. The largest differences, 0.029 and 0.0087, are isolated pixels of the two cases with a polynomial piece and no norm. A render's time is that of every pass, GPU timestamps: the reconstruction's 71 whole-image passes over 3908×2602, each reading or writing an image-sized buffer of 16 bytes a pixel, take about 7 ms each; the tone mapping alone takes 3 to 7 ms.

## Purpose

Filmic rgb is one of darktable's three scene-referred tone mappers, beside sigmoid and AgX: it takes scene light, unbounded and with middle grey near 18.45 %, to the display, compressing a chosen dynamic range around middle grey into it with a film-like S curve, keeping colours from clipping into flat primaries, and optionally rebuilding clipped highlights first.

Algorithmically, a logarithmic encoding maps the scene's black and white relative exposures to [0, 1]; a spline, linear over a latitude around middle grey with polynomial or rational toe and shoulder, maps that to display values, which a power ("hardness") takes to linear display light. Five colour sciences say how the curve meets colour: per channel or on a norm of the pixel, with a desaturation at the extremes (v3 to v5), or with a gamut mapping in Filmlight's Yrg against the display (v6, v7). Highlight reconstruction inpaints clipped areas with noise and diffuses it with an à trous B-spline wavelet decomposition, in RGB and then on the pixels' ratios.

In the first pass it is the Filmic RGB module, a tone mapper of its own in the Base tab, one tone mapper per edit (decision 2; `docs/modules/README.md`, Filmic RGB).

## Pipeline

- **darktable position:** 46.0 in the v5.0 order (`src/common/iop_order.c`, `v50_order`): after rgb levels (43), base curve (44), filmic (45), sigmoid (45.3) and AgX (45.5), before lut3d (listed after it) and colisa (47).
- **Apertrust slot:** working space 46.0, darktable's place: after AgX (45.5), the last of the tone mappers, before local contrast (display 54), as `docs/engine.md`'s Order table has it (decision 22).
- **Domain:** scene-referred in, display-referred out, as AgX and sigmoid (`Domain::Scene` in the manifest, AgX's choice).
- **Input and output:** working RGB, linear Rec2020, in and out; alpha passes through. The output is linear display light: the curve's black and white targets as its limits, middle grey at the grey target.
- **Assumes earlier modules:** demosaic, white balance or colour calibration, and exposure placing middle grey near the grey source. Negative input (out-of-gamut camera colours) is handled per colour science (Per pixel).
- **Layer support:** background only (`layers: false`) until layers come. The background's tone mapper runs once and its parameters are global. Planned with layers: a layer may choose its own tone mapper, filmic rgb among them, which replaces the background's within its mask, never stacked on it (decision 33).

## Operation

| Manifest field | Value |
| --- | --- |
| `id` | `filmicrgb` |
| `version` | 1 |
| `slot` | working space, 46.0 |
| `domain` | scene |
| `layers` | false |
| `differentiable` | true: `filmicrgb_backward` and `filmicrgb_forward_diff`, the derivatives of `main`, the pass that writes the output (Gradients) |
| `radius` | 0: the reconstruction's passes see the whole image (`PassKind::Reduction`, decision 15's whole-image kind under its code name), and `main` reads only its own pixel and their scratch |
| `default_enabled` | false: planned only once an edit can choose it as its tone mapper (decision 2) |
| `provenance` | darktable, commit 20891f6c6f; `src/iop/filmicrgb.c`, `src/common/bspline.h`, `src/develop/noise_generator.h`, `src/iop/gaussian_elimination.h`, `src/common/gamut_mapping.h`, `src/common/colorspaces_inline_conversions.h`, `src/common/iop_profile.h`, `src/common/math.h` |

`prepare` gives `main` alone while highlight reconstruction is off, and otherwise the reconstruction's whole-image passes before it (GPU plan). `main` binds the same resources either way, so one pair of derivative entry points serves every setting.

## Parameters

darktable params version 6, 116 bytes: eighteen `float`s, then eleven four-byte enums, booleans and integers, in this order. `DT_MODULE_INTROSPECTION(6, …)`: version 6 is the reference release's current one. darktable's "v7" is a colour science (`version`, "v7 (2023)"), not a params version. The schema (version 1) has darktable's field names, its introspection ranges (`$MIN`, `$MAX`) and its defaults (`$DEFAULT`); `Settings::default` holds them, and a test checks the schema against it. `reload_defaults` then moves three of them for a raw (Defaults).

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| Middle grey luminance | `grey_point_source` | f32, % | 0 to 100 | 18.45 | The scene grey mapped to the grey target; read only with custom middle-grey values | Filmic RGB: Scene |
| Black relative exposure | `black_point_source` | f32, EV | −16 to −0.1 | −8 | The scene black, below middle grey, the log encoding's 0 | Filmic RGB: Scene |
| White relative exposure | `white_point_source` | f32, EV | 0.1 to 16 | 4 | The scene white, above middle grey, the log encoding's 1; also the reconstruction's threshold reference | Filmic RGB: Scene |
| Threshold | `reconstruct_threshold` | f32, EV | −6 to 6 | 0 | Where the reconstruction mask reaches half, relative to the scene white | Filmic RGB: Reconstruct |
| Transition | `reconstruct_feather` | f32, EV | 0.25 to 6 | 3 | The mask's softness: its logistic's slope is `2^(12/feather)` | Filmic RGB: Reconstruct |
| Bloom ↔ reconstruct | `reconstruct_bloom_vs_details` | f32, % | −100 to 100 | 100 | The share of the wavelets' detail added back (100: all of it, −100: none, a pure bloom) | Filmic RGB: Reconstruct (Balance) |
| Grey ↔ colourful details | `reconstruct_grey_vs_color` | f32, % | −100 to 100 | 100 | The share of each channel's own detail against their common grey detail | Filmic RGB: Reconstruct (Balance) |
| Structure ↔ texture | `reconstruct_structure_vs_texture` | f32, % | −100 to 100 | 0 | The share of the unblurred texture against the blurred structure | Filmic RGB: Reconstruct (Balance) |
| Dynamic range scaling | `security_factor` | f32, % | −50 to 200 | 0 | Not read by processing: the interface scales the black and white exposures by it | Filmic RGB: Scene |
| Target middle grey | `grey_point_target` | f32, % | 1 to 50 | 18.45 | The display grey; read only with custom middle-grey values, and by Auto adjust hardness | Filmic RGB: Display |
| Target black luminance | `black_point_target` | f32, % | 0 to 20 | 0.01517634 | The display black | Filmic RGB: Display |
| Target white luminance | `white_point_target` | f32, % | 0 to 1600 | 100 | The display white | Filmic RGB: Display |
| Hardness | `output_power` | f32 | 1 to 10 | 4 | The power from the curve's display values to linear display light | Filmic RGB: Look |
| Linear region | `latitude` | f32, % | 0.01 to 99 | 0.01 | How much of the display range the curve's straight middle covers | Filmic RGB: Look |
| Contrast | `contrast` | f32 | 0 to 5 | 1 | The straight middle's slope (in spline v3, the slope at grey in linear display light, per 8 EV) | Filmic RGB: Look |
| Saturation | `saturation` | f32, % | −200 to 200 | 0 | By colour science: v3 and v6 extreme luminance saturation, v4 and v5 mid-tones saturation, v7 highlights saturation mix | Filmic RGB: Look |
| Shadows ↔ highlights balance | `balance` | f32, % | −50 to 50 | 0 | Slides the straight middle along its slope | Filmic RGB: Look |
| Add noise in highlights | `noise_level` | f32 | 0 to 6 | 0.2 | The inpainted noise's level, relative to the threshold | Filmic RGB: Options |
| Preserve chrominance | `preserve_color` | enum | 0 no, 1 max RGB, 2 luminance Y, 3 RGB power norm, 4 RGB euclidean norm (legacy), 5 RGB euclidean norm | 3 | The norm the curve applies to, keeping the pixel's ratios; "no" applies it per channel. darktable's combobox hides 4 | Filmic RGB: Options |
| Colour science | `version` | enum | 0 v3 (2019), 1 v4 (2020), 2 v5 (2021), 3 v6 (2022), 4 v7 (2023) | v7 | Per pixel, below | Filmic RGB: Options |
| Auto adjust hardness | `auto_hardness` | bool | | true | Not read by processing: the interface sets the hardness from the exposures and grey target | Filmic RGB: Options |
| Use custom middle-grey values | `custom_grey` | bool | | false | Whether the grey source and target apply; off, 18.45 % both | Filmic RGB: Options |
| Iterations of high-quality reconstruction | `high_quality_reconstruction` | int | 0 to 10 | 1 | Passes of the reconstruction on ratios after the one in RGB | Filmic RGB: Options |
| Type of noise | `noise_distribution` | enum | 0 uniform, 1 gaussian, 2 poissonian | gaussian | The inpainted noise's distribution | Filmic RGB: Options |
| Contrast in shadows | `shadows` | enum | 0 hard (4th-order polynomial), 1 soft (3rd-order), 2 safe (rational) | hard | The toe's curve | Filmic RGB: Options |
| Contrast in highlights | `highlights` | enum | as `shadows` | hard | The shoulder's curve | Filmic RGB: Options |
| | `compensate_icc_black` | bool | | false | Not read by the reference release at all, and no widget | fixed (`docs/modules/README.md`) |
| | `spline_version` | enum | 0 v1 (2019), 1 v2 (2020), 2 v3 (2021) | v3 | How the nodes follow from contrast and latitude; no widget | fixed at v3 (`docs/modules/README.md`, rule 12) |
| Enable highlight reconstruction | `enable_highlight_reconstruction` | bool | | false | | Filmic RGB: Reconstruct |

The schema's choice names come from darktable's descriptions, as sigmoid's do: `none`, `max_rgb`, `luminance_y`, `rgb_power_norm`, `rgb_euclidean_norm_legacy`, `rgb_euclidean_norm`; `v3_2019`, `v4_2020`, `v5_2021`, `v6_2022`, `v7_2023`; `uniform`, `gaussian`, `poissonian`; `hard`, `soft`, `safe`; `v1_2019`, `v2_2020`, `v3_2021`.

**Defaults** (`reload_defaults`). A new instance starts from the introspection defaults, but on an image whose matrix correction darktable supports (a raw, not monochrome) in a scene-referred workflow (every Apertrust photo is), darktable moves three of them by the camera's exposure bias b (the EXIF value, 0 outside ±5 EV, `dt_image_get_exposure_bias`): with `e = 0.7 − b`, `black_point_source = −8 + 0.5 e`, `white_point_source = 4 + 0.8 e`, and the hardness from them (`_compute_output_power`): `clamp(ln(grey_point_target / 100) / ln(−black / (white − black)), 1, 10)`. At b = 0: −7.65 EV, 4.56 EV and 3.6148162, as tests 0148 and 0153 store them. `Operation::defaults` does the same; other images keep −8, 4 and 4.

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 | 12 floats (`grey_point_source`, `black_point_source`, `white_point_source`, `security_factor`, `grey_point_target`, `black_point_target`, `white_point_target`, `output_power`, `latitude`, `contrast`, `saturation`, `balance`) and `preserve_color` | Shadows hard, highlights soft; threshold +6 EV (beyond any white), feather 3, bloom 100, grey ↔ colour 100, structure ↔ texture 0; colour science v3 (2019); auto hardness and custom grey on; high-quality iterations 0; gaussian noise at level 0; spline v1, converted to v3 (below); ICC black off; highlight reconstruction on |
| 2 | Adds the reconstruction's five floats after the white source, colour science, auto hardness, custom grey, iterations, shadows and highlights | Noise gaussian at level 0, spline v1 converted to v3, ICC black off, highlight reconstruction on |
| 3 | Adds `noise_level` and `noise_distribution` | Noise reset to gaussian at level 0.2, spline v1 converted to v3, ICC black off, highlight reconstruction on |
| 4 | Adds `compensate_icc_black` and `internal_version`, a year | `spline_version` from the year (2019 v1, 2020 v2, 2021 v3; any other refused), converted to v3; highlight reconstruction on |
| 5 | `internal_version` becomes the `spline_version` enum | Copied, spline version kept as it is; highlight reconstruction on |
| 6 | Appends `enable_highlight_reconstruction` | Current |

**`convert_to_spline_v3`**, for versions 1 to 4: the spline of the old settings (Setup) gives the new ones. With the nodes' `toe_log = min(x₁, x₂)`, `shoulder_log = max(x₃, x₂)` and display values likewise against `y₂`, the safety margin `m = 0.01 (y₄ − y₀)`: `contrast = (shoulder_display − toe_display) / (shoulder_log − toe_log)`; a toe below `y₀ + m` or a shoulder above `y₄ − m` is moved to it along that slope; then `contrast ·= 8 / (white_source − black_source)` and `·= hardness · y₂^(hardness − 1)`; `latitude = clamp((shoulder_display − toe_display) / (y₄ − y₀ − 2m), 0, 0.99)`; the balance from where the shoulder (or the toe) sits against the latitude's reference positions, `±0.5 (1 − …)`; for spline v1 also the black and white targets, `y^hardness · 100`, since v1 did not take the targets through the power. The fields are written in percent (`latitude`, `balance` times 100).

`ports.rs` migrates every version as darktable does: test 0014 is version 1, 0037 version 4 (internal version 2020), 0092 version 5, the others version 6. Since the migration converts every spline to v3 but version 5's, a spline v1 or v2 reaches processing only through a version 5 edit (darktable 3.6 to 4.0), never from darktable's interface, which has no widget for it: `prepare` refuses it (`NotPorted`, rule 12 of `docs/modules/README.md`), with a test. The spline code for v1 and v2 is still ported, since the migration needs it.

## Setup on the CPU (`commit_params`)

In f32 as darktable computes it, but where darktable uses doubles.

**Greys.** With custom middle-grey values, `grey_source = grey_point_source / 100` (at least 10⁻⁵ in v6 and v7 and for the reconstruction, a guard; a grey source of 0 in v3 to v5 encodes every pixel to 1, as in darktable: Precision), and `grey_display = (grey_point_target / 100)^(1/hardness)`; otherwise 0.1845 and `0.1845^(1/hardness)`. `commit_params` takes the grey target unclamped; the spline (below) clamps it between the black and white targets, so with a grey target outside them the two differ, as in darktable.

**The log encoding.** `dynamic_range = white_source − black_source` (EV), `black_source` as given; a scene value x encodes to `(log₂(x / grey_source) − black_source) / dynamic_range`, so black at 0, grey at `grey_log = |black_source| / dynamic_range`, white at 1.

**The spline** (`dt_iop_filmic_rgb_compute_spline`). Display targets: spline v1 takes `black_display = clamp(black_target, 0, grey_target) / 100` and `white_display = max(white_target, grey_target) / 100` as they are; v2 and v3 raise both to `1/hardness`. `balance = clamp(balance, −50, 50) / 100`.

- **Spline v1 and v2:** `latitude = clamp(latitude, 0, 100) / 100 · dynamic_range`, `contrast = clamp(contrast, 1.00001, 6)` (`commit_params` also raises a contrast below `grey_display / grey_log` to `1.0001 ·` it, into `d->contrast`, which nothing reads); `toe_log = grey_log − latitude / dynamic_range · |black_source / dynamic_range|`, `shoulder_log = grey_log + latitude / dynamic_range · |white_source / dynamic_range|`; the display values on the line of slope `contrast` through grey; then the balance shifts all four along the line by `−2 · latitude / dynamic_range · balance` over `√(contrast² + 1)`.
- **Spline v3**, darktable's for every new instance: `slope = contrast · dynamic_range / 8`; the least contrast that reaches both targets, `max(1, (white_display − grey_display) / (1 − grey_log), (grey_display − black_display) / grey_log) + 0.01`; the line's slope `contrast = clamp(slope / (hardness · grey_display^(hardness − 1)), that, 100)`, so the slope at grey after the power is the user's; `xmin` and `xmax` where the line reaches the targets less 1 % of the display range; `toe_log = (1 − latitude) grey_log + latitude · xmin`, likewise the shoulder with `xmax`; the balance moves both by `2 · balance · (shoulder_log − grey_log)` when positive, `2 · balance · (grey_log − toe_log)` otherwise, clamped to `[xmin, xmax]`; display values on the line.

The nodes: `x = (0, toe_log, grey_log, shoulder_log, 1)`, `y = (black_display, toe_display, grey_display, shoulder_display, white_display)`; `latitude_min = x₁`, `latitude_max = x₃`. The middle: `y = M1₂ + M2₂ x` with `M2₂ = contrast`, `M1₂ = y₁ − contrast · x₁`. The toe by `shadows`, the shoulder by `highlights`:

- **hard**, a 4th-order polynomial: the toe through `(0, y₀)` with slope 0 and through its node with the line's slope and zero curvature; the shoulder through `(1, y₄)` with slope 0 and through its node likewise. A 5×5 system solved by `gauss_solve` in f64 (Gaussian elimination with partial pivoting, in darktable's order of operations), the coefficients stored as f32.
- **soft**, a 3rd-order polynomial: through the end point, and through the node with the line's slope and zero curvature; 4×4, likewise.
- **safe**, rational: `y = M4 ∓ M1 · r / (r + M3)`, `r = ξ (ξ M2 + 1)`, `ξ` the distance from the node; from `x = Δlog`, `y = Δdisplay`, `g = contrast`: `b = g / 2y + (√((x g / y + 1)² − 4) − 1) / 2x`, `c = (y / g) (b x² + x) / (b x² + x − y / g)`, `a = c g`, in f32.

**Coefficients for `process`.** By colour science, `saturation = p / 100` for v6 and v7, `2 p / 100 + 1` for v3 to v5; `sigma_toe = (latitude_min / 3)²`, `sigma_shoulder = ((1 − latitude_max) / 3)²`. `process` adds `display_black = y₀^hardness`, `display_white = y₄^hardness`, and for v6 and v7 `norm_min = grey_source · 2^black_source` and `norm_max = grey_source · 2^(black_source + dynamic_range)`, the scene values that encode to 0 and 1.

**Reconstruction coefficients.** `threshold = 2^(white_source + reconstruct_threshold) · grey_source`, `feathering = 2^(12 / reconstruct_feather)`, `normalize = feathering / threshold`; each balance `(p / 100 + 1) / 2`, so 0 % is 0.5.

**Matrices** (v6 and v7, `prepare_RGB_Yrg_matrices`), in f32 as darktable multiplies them: pipeline RGB to CIE 2006 LMS, `XYZ_D65_to_LMS_2006 · CAT16(D50→D65) · RGB_to_XYZ_D50`, and back, `XYZ_D50_to_RGB · CAT16(D65→D50) · LMS_2006_to_XYZ_D65`; and the same for the pipeline's output profile (Deliberate differences). The luminance of v3 to v5 is the working profile's Y row of RGB to XYZ D50 (`dt_ioppr_get_rgb_matrix_luminance`; linear Rec2020 has no tone curve).

## Per pixel (`process`)

The tone mapping's input is the reconstructed image where highlight reconstruction ran (next section), the operation's input otherwise. `spline(x)` is `filmic_spline`: the toe below `latitude_min`, the shoulder above `latitude_max`, the line between, in Horner's form; the kernel evaluates the hard and soft pieces around their nodes (Deliberate differences). `log_v1(x) = clamp((log₂(x / grey) − black) / dr, 2⁻¹⁶, 1)`, `log_v2` the same clamped to [0, 1]. `pow_dt(x, p)` is `dt_vector_powf`: `exp2_dt(p · log2_dt(x))`, darktable's polynomial approximations, the mantissa's degree-5 minimax for log₂ and a degree-4 one for the fraction of 2ˣ after rounding `x − 0.5` half away from zero, reproduced to the bit pattern (Precision). The norms (`get_pixel_norm`): max RGB; luminance Y; the power norm `Σ|c|³ / max(Σc², 10⁻¹²)`; the euclidean norm `√Σc²`, and the non-legacy one times `1/√3`; "no" falls back to luminance where a norm is still needed.

**v3 (2019), `filmic_split_v1` and `filmic_chroma_v1`.**

- No chrominance preservation: `t = log_v1(max(c, 2⁻¹⁶))` per channel; `lum = Y · t`; `desat = 1 − clamp((e^(−lum²/2σt) + e^(−(1−lum)²/2σs)) / saturation)`; per channel `spline(lum + desat (t − lum))`, clipped to [0, 1], then `pow_dt` of the hardness.
- With a norm: `n = max(norm(c), 2⁻¹⁶)`; ratios `c / n`, less their minimum where it is negative; `n = log_v1(n)`; `desat` of n; `r ·= n`, `lum = Y · r`, `r = (lum + desat (r − lum)) / n`; `n = clamp(spline(n))^hardness` (`powf`); output `r · n`.

**v4 (2020) and v5 (2021), `filmic_split_v2_v3` and `filmic_chroma_v2_v3`.**

- No preservation: as v3's but `log_v2` and the v2 desaturation, `desat = s − (e^(−lum² / σt · k) + e^(−(1−lum)² / σs · k)) s` with `k = 0.5 / √s`, `s` the saturation coefficient.
- With a norm: ratios and their sanitising as v3's; `n = log_v2(n)`; `desat` of n; `n = clip(spline(n))^hardness` (`powf`); `r = max(r + (1 − r)(1 − desat), 0)`; v5 divides n by the norm of the new ratios (at least 2⁻¹⁶); output `r · n`; where its largest channel m is over 1, `r = max(r + 1 − m, 0)` and the output `clip(r · n)`.

**v6 (2022), `filmic_split_v4` and `filmic_chroma_v4`.**

- Tone mapping, no preservation (`RGB_tone_mapping_v4`): per channel `spline(log_v2(c))`, a channel at or below `norm_min` encoding to 0, clamped to `[0, display_white]`, then `pow_dt` of the hardness.
- With a norm (`norm_tone_mapping_v4`): `n = clamp(norm(c), norm_min, norm_max)`, ratios `c / n` (unsanitised), `n = clamp(spline(log_v2(n)), display_black, display_white)^hardness` (`powf`; darktable clamps the curve's value, before the power, to the bounds after it), output `r · n`.
- Then both: the input's and the output's Ych in Yrg (`RGB_to_Ych`: pipeline RGB to LMS 2006, Yrg, polar), without preservation the output's chroma limited to the input's, and the gamut mapping below with the saturation coefficient.

**v7 (2023), `filmic_v5`**, darktable's default, whatever the norm chosen: both tone mappings above, the per-channel one and the max RGB norm's, mixed `(0.5 − s)·per-channel + (0.5 + s)·max RGB` with `s = saturation / 100`; the output's chroma limited to the input's; the gamut mapping with saturation 0.

**The gamut mapping** (`gamut_mapping`, v6 and v7), on the output's Ych:

1. The input's hue; luminance clamped to `[1.05785528 · display_black, 1.05785528 · display_white]` (CIE 1931 Y to 2006 Y).
2. `filmic_desaturate_v4`: chroma as `C·Y` of the input (`c₂`) and output (`c₁`); if the curve brightens and resaturates, their mean; else, where the user resaturates (`s > 0`) and the curve desaturated, or the user desaturates (`s < 0`), `c₁ + s (c₂ − c₁)`; else `c₁`; the new chroma `max(c / Y, 0)`.
3. `gamut_check_Yrg`: chroma clipped at constant Y and hue to stay inside Yrg's triangle.
4. `gamut_check_RGB` in the output profile's RGB (Deliberate differences): the colour at that Ych, brightened by the white light that lifts its lowest channel to 0, gives a luminance `Y = clamp(mean of the two, bounds as above)`; the chroma limited to `Ych_max_chroma` at that Y and hue, the most each channel of the output RGB allows before it passes `display_white` or 0 (`gamut_mapping.h`, darktable's closed form per channel, linearly to 0 within 10⁻³ of the white); back to RGB, clamped to `[0, display_white]`; then through LMS to the pipeline's RGB.

**At the defaults the operation is no identity**: it is a tone mapper. Middle grey at the defaults (v7, custom grey off) goes to the grey target 18.45 % within the curve's precision.

The kernel reads every channel it chooses at run time through one-hot vectors and dot products, and never writes one at a run-time index (decision 10).

## Highlight reconstruction (`process`, `reconstruct_highlights`)

Runs when enabled and more than 9 pixels are clipped; otherwise the tone mapping reads the input. Everything is over the whole image at the pipe's scale (Neighbourhood and scale), i and j the pixel's row and column.

1. **Mask** (`mask_clipped_pixels`): `w = clamp(1 / (1 + 2^(feathering − normalize · ‖c‖)))`, `‖c‖` the euclidean norm of RGB; a pixel counts as clipped where `feathering − normalize · ‖c‖ < 4` (opacity over 5.9 %).
2. **Noise** (`inpaint_noise`): per pixel a xoshiro128+ generator seeded by `splitmix32(j + 1)`, `splitmix32((j + 1)(i + 3))`, `splitmix32(1337)`, `splitmix32(666)` (64-bit splitmix arithmetic) and advanced four times; noise around each channel with `σ = c · noise_level / threshold`, `noise_level` divided by the pipe's downscaling (`max(iscale / roi→scale, 1)`): uniform `c + 2(u − ½)σ`; gaussian by Box–Muller, cosine for red and blue and sine for green, `u₁ = max(u, FLT_MIN)`, three u₁ then three u₂; poissonian the gaussian's noise through an Anscombe transform, `r = nσ + 2√max(c + ⅜, 0)`, `(r² − σ²)/4 − ⅜`, u₁ and u₂ drawn in turn. `inpainted = max(c (1 − w) + w · noise, 0)`.
3. **RGB pass** (`reconstruct_highlights`, RGB): `reconstructed = max(inpainted (1 − w), 0)`; then for each scale s from 0 to `scales − 1` with step `2^s`: the low frequencies `LF = blur(detail, 2^s)` (`blur_2D_Bspline`, the B-spline `[1 4 6 4 1] / 16` vertically then horizontally, taps at `±2^s` and `±2^(s+1)`, clamped at the edges, each 1-D sum clipped at 0), `HF = detail − LF`, the next scale's detail being LF; `HF_RGB = blur(HF, 1)`, also clipped at 0 (darktable's FIXME keeps it); and `reconstructed += w (δ · details + residual)`, with `gamma`, `beta` and `δ` the three balances (structure ↔ texture, grey ↔ colour, bloom ↔ reconstruct), `grey_texture` the channel of HF largest in magnitude, `grey_details` HF_RGB's mean, `details = (γ' HF_RGB + γ HF) β + β' (γ' grey_details + γ grey_texture)`, the residual at the last scale only, `β' · mean(LF) + β · LF`. Primes are `1 −`.
4. **Ratio passes**, `high_quality_reconstruction` times: the euclidean norm `n = max(‖reconstructed‖, 2⁻¹⁶)` and ratios `reconstructed / n`; the same wavelet pass on the ratios (`wavelets_reconstruct_ratios`: `details = ½ ((γ' HF_RGB + γ HF) + γ' grey_details + γ grey_texture)`, the residual LF itself); `reconstructed = clamp(reconstructed, 0, 1) · n`.

**Scales** (`get_scales`): `⌊log₂(2 · size · scale / 20 − 1)⌋` clamped to [1, 10], with `size` the image's longer side at full resolution and `scale` the pipe's; so the coarsest filter covers about the same fraction of the image at every scale. 3908×2602 at full resolution: 8; 5492×3672: 9.

## GPU plan

Every pass is one invocation per pixel of the whole image at the pipe's scale, `[numthreads(16, 16, 1)]`, but the count's combination. Scratch bindings: 10 the mask, one float a pixel; 11 the clipped counts, the total then one partial per workgroup; 12 the detail (inpainted noise, then each scale's detail and its high frequencies, then the ratios), 13 the low frequencies, 14 the vertical blur, 15 the reconstructed image, four floats a pixel each; 16 the norms, one float a pixel.

| Pass | Kind | Reads | Writes |
| --- | --- | --- | --- |
| `hr_mask` | whole-image | input | 10, and 11's partial per workgroup |
| `hr_count` | whole-image, one workgroup | 11's partials | 11's total |
| `hr_inpaint` | whole-image | input, 10 | 12 |
| `hr_init`, once per reconstruction | whole-image | 12, 10 | 15 |
| `hr_blur_v`, twice per scale (the detail at `2^s`, then HF at 1) | whole-image | 12 | 14 |
| `hr_blur_h`, once per scale | whole-image | 14, 12 | 13 (LF), 12 (`HF = detail − LF`, in place) |
| `hr_reconstruct`, once per scale | whole-image | 14 (its horizontal blur is HF_RGB), 13, 12, 10 | 15 |
| `hr_ratios`, per iteration | whole-image | 15 | 12, 16 |
| `hr_restore`, per iteration | whole-image | 15, 16 | 15 |
| `main` | per pixel | the pixel, 11's total and 15 at the pixel when reconstruction is on | the pixel |

Bindings 12 and 13 swap scratch buffers from scale to scale, as darktable's `LF_even` and `LF_odd`, so HF overwrites its detail in place: buffer A holds the inpainted image (or the ratios), the detail of scale 0, and then of every even scale, buffer B every odd scale's. For 3908×2602, 8 scales and one iteration: 3 + (1 + 4·8) + (2 + 1 + 4·8) + 1 = 72 passes, and 72 bytes a pixel of scratch, 732 MB; darktable's own reconstruction keeps about `(7.25 + 1.25) · 16` bytes a pixel (`tiling_callback`). Each pass's uniform holds only what it reads (decision 15): the reconstruction's settings never reach `main`'s bytes, and the curve's never reach theirs (a unit test).

While reconstruction is off, `prepare` gives `main` alone, with two scratch buffers of 16 bytes for the bindings it then never reads.

**The reconstruction's cost:** 71 whole-image passes over image-sized buffers, about 0.55 s a render of 3908×2602 on the M4. It is measured when the Filmic RGB module is built (Phase 3): each scale's vertical blurs are fused if that pays, and the passes are skipped when the count of clipped pixels is under darktable's 10, read through a readout (decision 19).

## Neighbourhood and scale

- **Neighbourhood radius:** the reconstruction's wavelets reach `2 (2^scales − 1) + 2` pixels per scale chain, 512 at mire1's 8 scales, and its count is the whole image's; the tone mapping is per pixel.
- **Scale dependence:** the number of scales follows the pipe's scale (get_scales), the steps do not; the noise level is multiplied by the pipe's scale; the noise's seeds are the pixel's row and column in the image at the pipe's scale.
- **What Apertrust computes at each scale** (decision 15; principle 2): the reconstruction's passes run over the whole image at the pipe's scale in every pipe, never a visible region, with darktable's fewer scales and scaled noise there. At scale 1, an export and the 100 % view, they are darktable's export exactly. Decision 15's viewport target holds with room to spare (`the_viewport_is_within_decision_80s_target_of_the_export`, ignored, run by hand): on the five cases whose reconstruction runs, the viewport's mean ΔE2000 against the export downscaled is 0.048 to 0.085 at 2400×1600, 0.069 to 0.12 at 1/2, 0.13 to 0.25 at 1/4 and 0.22 to 0.38 at 1/8, and the reconstruction adds at most 0.0003 to the same comparison with it off, the baseline: the gap is the tone mapping's own, a non-linear curve of a box filter's mean against the mean of the curve.
- **darktable's tiling:** `tiling_callback` gives an overlap of `2^scales`, less than the wavelets' reach, and each tile counts its own clipped pixels and seeds its noise by its own coordinates; darktable tiles only when its memory runs short, and the golden renders do not tile.
- **Tiling in Apertrust:** filmic rgb does not opt in to a first round of tiles (`Operation::tiles_whole_image_passes`): the wavelets need the whole image, so an export runs it whole with everything before it, and tiles only what follows, in one round. A test holds a tiled export bit-identical to the whole one. Under decision 7 the reconstruction is to become one run that opts in to a first round of export tiles, to be measured on the RTX 5060 (planned).
- **lavapipe's 128 MiB binding:** the reconstruction's 72 bytes a pixel of scratch, about 4.4 GB at 61 MP, are refused on lavapipe above about 8.4 MP (each float4 buffer is 16 bytes a pixel), about 37 MP once decision 15's windows exist. The refusal is accepted, an exception on decision 7's list. The remedy is transposed vertical blurs.
- **Region-of-interest changes:** none.

## Precision and special values

- **fp64:** darktable's spline solve is f64 (`gauss_solve`), its coefficients stored as f32; the port solves the same way on the CPU, keeping darktable's f32 monomials for the migration and handing the kernel the pieces around their nodes (Deliberate differences). Everything else in darktable is f32, and so is the port's setup, with `powf` as f64's `pow` rounded once (`docs/ports/sigmoid.md`, Precision). The migration of test 0014's version 1 settings gives darktable's own (test 0139's first entry) within 4 ulps.
- **darktable's `pow` is approximate** where the curve's display values go to linear light per channel (`dt_vector_powf` in v3 to v5 without preservation, and in v6 and v7's per-channel tone mapping): `log2_dt` is within 5.7e-5 of log₂ and `exp2_dt` within 2.6e-6 relative of 2ˣ on its [0, 1] fraction, so `pow_dt` is up to about 1.6e-4 relative from `pow` at hardness 4, about 0.01 ΔE2000 in mid-tones. The kernel reproduces it from the float's bits, with derivatives of the approximation itself, so finite differences agree with them. Where darktable calls `powf`, the kernel calls the GPU's `pow`. It is the arm64 build's `pow_dt` bit for bit, the reference build: arm64 has no SSE2, so `dt_vector_round` is `roundf`, half away from zero, while an x86 build's `_mm_cvtps_epi32` rounds half to even. They part where `x − 0.5` is a half, at an integer exponent: at 0, x86 takes a fraction of 0 and gives 1.0000026, arm64 a fraction of 1 and 2⁻¹ · fit(1), 0.9999974; so where the curve's value is 1, a Linux or Windows darktable's white is 5.2e-6 above the macOS one.
- **FMA:** darktable's arm64 build fuses multiply-adds through the module (574 fused multiply-add instructions, scalar and vector, in its `libfilmicrgb.so`); the kernel leaves fusing to the GPU's compiler, a difference in the last bits.
- **The noise** is computed with the GPU's `log`, `cos`, `sin` and `sqrt`, darktable's with the C library's: the same uniform draws to the bit, gaussian and poissonian noise within the functions' precision, which the wavelets then blur.
- **Divisions and NaN, where darktable's result is reproducible the kernel renders it** (darktable-cli renders on mire1):
  - **The saturation coefficient** for v3 to v5 reaches 0 at −50 % and is negative below. v3's desaturation divides by it: its clamp takes ±∞ (or 0 / 0) to a defined limit, which the kernel's division by 10⁻³⁰ of the coefficient's sign gives too. v4 and v5 take its square root: below 0 the desaturation is NaN, which `dt_vector_clip` of the curve's value (no norm) and `MAX(…, 0)` of the ratios (a norm) take to 0, so darktable renders a uniform black (−9.4e-10 on mire1); the kernel renders 0 there in both paths, by the coefficient's sign, with no NaN. At 0 itself, v4 and v5's `0.5 / √0` is infinite, and the kernel takes its limit, the desaturation's 0.
  - **A custom grey source of 0:** v3 to v5 divide every pixel by it, and the clamp of the log's infinity encodes every pixel to 1, a uniform white (v5: 0.9999999 on mire1). The setup flags it (`encodes_to_white`, the uniform's `flags.z`), and the kernel's log encodings return 1 without dividing. v6 and v7 render all NaN there (0 / 0 ratios), which is no result to follow: they, and the reconstruction's threshold in every version, take the grey source at 0.001 % at least (Deliberate differences).

  darktable's `log₂` of a non-positive channel in v6 and v7's per-channel tone mapping is −∞ or NaN, which its clip at 0 turns into 0; the kernel encodes every channel at or below `norm_min` to 0 directly. `Ych_max_chroma`'s `FLT_MAX` stays finite: a near-white limit that would scale it past `FLT_MAX` returns `FLT_MAX`, which the minimum discards either way.
- **The count** is an integer, as darktable's; summed per workgroup and combined, the same total. The mask's exponent is held within ±126 before `exp2`, where darktable's flush to zero takes the opacity to 0 (or 1) beyond it anyway.

## CPU and OpenCL differences in darktable

- `filmic_mask_clipped_pixels` sets a flag where any pixel clips, not more than nine; the CPU path needs ten.
- The OpenCL wavelets blur the low frequencies horizontally then vertically and the high frequencies vertically then horizontally, each 1-D pass clipped at 0 (`bspline.cl`), where the CPU blurs vertically then horizontally both times; with the clips in between, the results differ.
- `compute_ratios` on OpenCL ignores the variant and uses the euclidean norm, as the CPU's call does.
- The port follows the CPU path; the references are rendered on the CPU (`--disable-opencl`).

## Deliberate differences

- **The output profile is the working profile.** darktable's v6 and v7 gamut-map against the pipe's output profile where it is a matrix profile (`dt_ioppr_get_pipe_output_profile_info`), and then convert back to the pipeline's RGB. The image context carries the pipeline's output profile (decision 26: colorout reports it through `Operation::output_profile`), but filmic does not read it yet, so `prepare` gamut-maps against the working profile, linear Rec2020. darktable does the same when the output profile is linear Rec2020, as in every golden render, so no case shows the difference.

  An sRGB export differs in saturated colours that leave sRGB's gamut but not Rec2020's. Measured with darktable-cli at test 0138's settings, darktable's sRGB export against its linear Rec2020 export converted to sRGB, ΔE2000: on mire1 a mean of 0.025 and a maximum of 10.15, 0.19 % of pixels over 2.3; on a Pentax K-3 III raw a mean of 0.21 and a maximum of 5.07; a Nikon Z 6 raw's maximum 6.40, a Canon EOS 5D Mark IV's 3.24. That is past `Tolerance::DARKTABLE`'s maximum, so filmic reading `ImageContext::output_profile` comes before an edit can choose Filmic RGB as its tone mapper (planned). `setup::prepare` takes the export profile's matrices, so only the operation's call changes.
- **The hard and soft pieces evaluated around their nodes.** darktable stores each polynomial piece's monomial coefficients in f32 and evaluates them in x, where near the shoulder terms of about 20 cancel to values near 1, so its rounding of the coefficients is noise of about 10⁻⁶ in display values, which finite differences of the settings magnify to percent-level errors. The kernel takes the same polynomial, darktable's f64 solve, as Taylor coefficients at its node (`Spline::taylor`): the node's display value, the line's slope, no curvature (the solve's own conditions there), and the solve's cubic and quartic terms, evaluated in `d = x − node`. It is closer to darktable than evaluating the f32 monomials: the two cases with a polynomial piece and no norm reach maxima of 0.029 and 0.0087, against 0.058 and 0.0135 with the monomials, every other case the same within 0.0001; and it makes the contrast's, the latitude's and the hardness's gradients agree with finite differences (Gradients). A unit test holds the two forms equal within 10⁻⁹ in f64.
- **A custom grey source below 0.001 % in v6 and v7, and in every version's reconstruction**, is taken at 0.001 %, the least darktable's own picker sets (`apply_auto_grey`): a guard. darktable's slider reaches 0, where v6 and v7 render all NaN and the reconstruction's threshold of 0 fills the image with NaN, no result to follow; between 0 and 0.001 % darktable's results are finite but extreme (a scene grey over 14 EV below 18.45 %), and the guard departs from them there. v3 to v5 render darktable's uniform white at 0 (Precision), and every grey source above 0 as darktable does.
- **Ych from the pixel scaled to its largest channel** (v6, v7): the same values within rounding and exactly the same derivatives, since Ych's chroma and hue do not depend on the scale and its Y is linear in it, where the backward pass would square the LMS sum of a pixel of 10⁻²⁰ and below past f32's range.
- **Saturation at and below −50 % in v3 to v5** is no difference: v3 keeps darktable's limit, and v4 and v5 render darktable's black below it and its limit at it (Precision); only the NaN darktable passes through on the way is not computed.
- **The B-spline sums, the noise's functions and the GPU's `pow`, `log₂` and `exp`** (Precision): the last bits.
- **Spline v1 and v2 are refused** (`NotPorted`, Parameter versions): only a version 5 edit holds them.
- **The whole image at every pipe's scale** (Neighbourhood and scale), where darktable's darkroom processes the visible region: principle 2's same processing at each scale, not a departure from darktable's export.
- **Not ported (GUI only):** the mask display, the graph, the pickers and auto tune, the auto hardness and the dynamic range scaling, which act on parameters in the interface; they belong to the Filmic RGB module.

## Tool binding notes

In the first pass the Filmic RGB module is darktable's page (`docs/modules/README.md`, Filmic RGB): content tabs Scene, Reconstruct, Look, Display and Options, darktable's controls under its labels.

- **Units:** the module stores darktable's values and units (EV, percentages as darktable stores them, the hardness as a power). Interface behaviours darktable runs on a change (`gui_changed`): with Auto adjust hardness on, the hardness follows `_compute_output_power` whenever the exposures, the grey or the scaling move; Dynamic range scaling scales both exposures by `(new − old) / (old + 100)`; a new grey source moves both exposures by `log₂(old / new)`; moving Threshold or Transition turns reconstruction on. They are the module's to reproduce, since the operation never reads `auto_hardness` or `security_factor`.
- **Defaults and presets:** a new instance's values are darktable's (`Operation::defaults`, with `reload_defaults`'s exposure shift on raws). darktable's only preset, "scene-referred default", auto-applied in its filmic workflow, stores the defaults (NULL params).
- **What is an identity:** none; a tone mapper needs none, since exactly one runs per edit (decision 2).
- **Derivatives:** four directions in parameter space, contrast, hardness, latitude and saturation (Gradients), the module's controls of the same names; a binding that wants others, the black or white exposure, needs them as directions, and the reconstruction's threshold held.

## Gradients

`main`'s derivative entry points differentiate the per-pixel tone mapping with respect to four directions in the schema's parameters, in this order: `contrast`, `output_power` (the hardness), `latitude` and `saturation`, the last two as fractions, the schema's percentages over 100 (`Direction::scale`), the kernel's own units. The kernel's coefficients (the spline's pieces, the log encoding, the saturation coefficient, the desaturation's sigmas and the bounds: 25 values) are `c₀ + Σₖ tₖ Dₖ`, with `Dₖ = ∂c/∂pₖ` from the CPU, central differences of the same setup in f64, and the entry points differentiate against t at 0, as the tone equalizer's band directions do (`docs/ports/toneequal.md`, Gradients). A unit test holds the directions to central differences of darktable's f32 setup. None of the four reaches the reconstruction's passes, whose threshold depends on the white exposure and grey source alone, so `main`'s derivatives are the operation's (decision 15).

**Why these four.** The black relative exposure is not a slot: spline v3 sets the contrast per EV at grey, so moving the black keeps most of the image where it was, and its derivatives are near-cancelling sums, 10⁻⁴ to 10⁻² of the contrast's over the image: f32 finite differences of whole renders resolve it only at steps of 1e-2 to 5e-2 EV, within a few percent in v6 and v7, and forward and reverse mode then differ per pixel by about 2.7e-4 relative to such small values. The white relative exposure and the grey source move the reconstruction's threshold, which `main`'s derivatives hold fixed. The balance has a kink at 0, where most edits sit.

**What each case names**, a prefix of the slots (`ports::gradient_parameters`): all four but three exceptions, each the same on the case and on the sweep, since the harness names a case's slots once.

- v6 (2022) at a saturation of 0 (`filmicrgb-v6`, `filmicrgb-v6-euclidean`): not the saturation, where `filmic_desaturate_v4` switches branch on its sign, a kink finite differences straddle (they average the slopes either side, autodiff takes the one at 0).
- v3 (2019) with a norm (`filmicrgb`): not the saturation, which desaturates each pixel's ratios towards their luminance, so the sweep's hues, grey on average, cancel it in the loss (on the case's own image it passes, within 1.2e-5).
- Test 0037 (`filmicrgb-reconstruction`): neither the latitude nor the saturation. The latitude's total on the chart is 3.6e-4 of the contrast's, so the 1% of pixels on the ratios' clamps at 0, saturated oranges with blue at or below 0 (pixel (2440, 616) is (0.588, 0.379, −0.012)), decide it: finite differences straddle those kinks, 46% from the derivative at a step of 1e-3 and about 1.3% at 3e-3 and 1e-2, while forward and reverse mode agree per pixel within 1.4e-6 and the same settings pass on the sweep (1.1e-3). The saturation, a later slot, goes with it, though it passes there (3.0e-5).

The harness names a case's slots as a prefix, and its `sweep_blind` list applies only to single-kernel cases, not to plans (`gradient_plan`). Planned: a harness change lets a case name any slots and plans use `sweep_blind`, so that v3's saturation with a norm, which only the sweep cannot see, and test 0037's saturation, which passes but follows a slot that does not, are checked too.

Results (Apple M4, step 1e-3 in each direction's units, the sum of output RGB as the loss): relative error of reverse mode against central finite differences of whole renders, and in brackets forward mode's per-pixel difference from reverse mode.

| Case | contrast | output_power | latitude | saturation |
| --- | --- | --- | --- | --- |
| `filmicrgb` | 2.6e-4 (1.0e-7) | 3.7e-3 (1.1e-6) | 2.1e-3 (7.9e-7) | not named |
| `filmicrgb-reconstruction` | 2.2e-4 (1.3e-7) | 5.4e-3 (1.1e-6) | not named | not named |
| `filmicrgb-v6` | 8.9e-5 (2.8e-7) | 1.8e-3 (1.2e-6) | 9.5e-4 (7.8e-7) | not named |
| `filmicrgb-v7` | 1.2e-4 (2.1e-7) | 3.8e-3 (7.1e-7) | 6.1e-4 (9.0e-7) | 1.3e-5 (1.5e-6) |
| `filmicrgb-v3-nochroma` | 5.8e-4 (8.9e-8) | 7.6e-3 (8.7e-7) | 5.8e-4 (6.3e-7) | 4.5e-5 (1.2e-7) |
| `filmicrgb-v4-nochroma` | 2.7e-4 (5.8e-7) | 2.8e-3 (4.4e-6) | 7.3e-5 (8.3e-7) | 2.4e-5 (9.1e-8) |
| `filmicrgb-v5-nochroma` | 9.5e-5 (1.2e-7) | 4.0e-3 (6.0e-7) | 7.0e-4 (5.4e-7) | 6.6e-5 (1.1e-7) |
| `filmicrgb-v6-euclidean` | 8.0e-5 (2.2e-7) | 4.0e-3 (5.8e-7) | 7.2e-4 (6.8e-7) | not named |
| synthetic, `filmicrgb` | 2.1e-4 (2.6e-7) | 3.2e-3 (1.4e-6) | 1.3e-3 (1.1e-6) | not named |
| synthetic, `filmicrgb-reconstruction` | 1.8e-4 (1.6e-7) | 3.7e-3 (8.5e-7) | not named | not named |
| synthetic, `filmicrgb-v6` | 2.4e-5 (2.3e-7) | 5.8e-3 (8.9e-7) | 4.5e-4 (7.8e-7) | not named |
| synthetic, `filmicrgb-v7` | 2.0e-4 (2.2e-7) | 4.0e-3 (9.9e-7) | 6.4e-4 (1.1e-6) | 1.1e-5 (4.5e-7) |
| synthetic, `filmicrgb-v3-nochroma` | 5.0e-4 (2.3e-7) | 4.1e-3 (8.6e-7) | 1.7e-4 (4.7e-7) | 4.8e-5 (1.1e-7) |
| synthetic, `filmicrgb-v4-nochroma` | 1.8e-4 (2.1e-7) | 7.9e-4 (6.4e-7) | 7.2e-4 (9.7e-7) | 5.5e-5 (1.1e-7) |
| synthetic, `filmicrgb-v5-nochroma` | 5.6e-5 (1.5e-7) | 3.6e-3 (6.9e-7) | 4.0e-4 (5.8e-7) | 4.2e-3 (1.0e-7) |
| synthetic, `filmicrgb-v6-euclidean` | 4.5e-5 (2.3e-7) | 3.5e-3 (7.4e-7) | 3.7e-4 (6.0e-7) | not named |

Every named parameter passes, below 1e-2 and 1e-4: `gradient` and `gradient --synthetic` exit 0 over the eight cases, at the harness's step and with no case's own.

Known non-smooth spots, where finite differences straddle a kink and per-pixel gradients jump: the spline's joins (C² for the polynomials at the nodes, C¹ for the safe pieces), the log encoding's clamps at 0 and 1, the clips of the display values, the norms' and the ratios' clamps and the sanitising of a negative channel, v6's saturation at 0, the gamut mapping's minimums and clamps, the contrast's clamp to its least value, the balance's sign and the nodes' clamps to `[xmin, xmax]`, and `pow_dt`'s segment boundaries, where its value is continuous and its slope jumps by the approximation's error.

## Test plan

- **Golden cases**, darktable's integration tests on mire1.cr2, sidecars unchanged, no later module to disable: `filmicrgb` (0014-filmic-rgb: params version 1, v3 colour science, power norm, migrated from spline v1, custom grey, reconstruction on but beyond every pixel), `filmicrgb-reconstruction` (0037: version 4, v4 colour science, power norm, soft toe and hard shoulder, reconstruction at +0.5 EV with one iteration and no noise), `filmicrgb-v6` (0092: version 5, v6 colour science, no preservation, safe curves, reconstruction at +3 EV, gaussian noise), `filmicrgb-v7` (0138: v7, reconstruction at +3 EV, poissonian noise, balance 5 %), `filmicrgb-v3-nochroma` (0139: v3, no preservation, safe toe, soft shoulder, custom grey), `filmicrgb-v4-nochroma` (0140: v4, no preservation, safe toe, hard shoulder), `filmicrgb-v5-nochroma` (0141: v5, no preservation, reconstruction off), `filmicrgb-v6-euclidean` (0142: v6, euclidean norm). Each `Tolerance::DARKTABLE`, each a one-instance plan of the engine (decision 15). Planned: darktable's tests 0148 (v7's gamut mapping at saturation 50 %) and 0153 (the reconstruction with six iterations on hlrecovery.arw), which use only what is ported.
- **Unit tests** (`crates/op/filmicrgb`): the schema is version 6's 29 fields in order with darktable's defaults; every parameter reaches its setting and round-trips; the defaults move with a raw's exposure bias as `reload_defaults` does (−7.65, 4.56, 3.6148162 at 0, as tests 0148 and 0153 store them), not for other images; `gauss_solve` against a known solution and a singular matrix; every pairing of the curves meets its nodes with the line's slope, the safe curves rising; the node-centred pieces equal darktable's monomials; spline v3 at the defaults takes grey to 18.45 %; spline v1 and v2 refused; the saturation coefficient per colour science; a grey source of 0 sets v3 to v5's white flag in `main`'s uniform and is floored in v6 and v7, and a grey source above 0 is darktable's; darktable's scales at several sizes and pipe scales; the passes, their order and count (72 for mire1), sizes, bindings and scratch for reconstruction on and off, buffers A and B swapping by scale, every pass within eight storage bindings, only `main` per pixel and writing the output, binding the same resources either way; the curve's settings reach only `main`'s bytes and the reconstruction's only theirs; the directions against central differences of darktable's f32 setup; `splitmix32` and `xoshiro128plus`; `dt_vector_powf`'s approximation within 2.5e-4 of `pow`; the uniforms' layout.
- **In `ports.rs`:** each case reads at its version, migrating as `legacy_params` does, spline v1 and v2 converted to v3; test 0014's version 1 settings migrate to test 0139's first entry, darktable's own migration of them, within 4 ulps; version 4's unknown years and other versions and sizes refused; the gradient slots each case names, and that each direction moves its parameter in its units.
- **On the GPU** (`crates/golden/tests/filmicrgb.rs`, run in CI): middle grey goes to the grey target within 0.2 % in every colour science, with every norm linear on greys and every curve; alpha passes through and greys rise, but for `pow_dt`'s ripple at the white; below its zero the saturation coefficient renders v4 and v5 black, with and without a norm, as darktable does, and at its zero middle grey stays bright; a grey source of 0 renders v3 to v5 a uniform white, as darktable does, and v6 and v7 finite; with nothing past the threshold the reconstruction renders exactly what it does off; a clipped block is rebuilt, with RGB and ratio passes and three noises, while far from it the image moves by less than 1e-3; output and both derivative modes are finite for black, 1e-30, negative, −1000, 10⁶ and saturated input at both ends of every float parameter's range, over every colour science, norm and curve (90 settings). And, ignored, run by hand: the viewport's target (Neighbourhood and scale). In `crates/engine/tests/tiles.rs`, run in CI: a tiled export through the default pipeline with filmic rgb as its tone mapper, reconstruction on at −3 EV, equals the whole one bit for bit at every tile size, in one round.
