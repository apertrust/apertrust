# Port spec: color equalizer

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/colorequal.c` `commit_params()`, `_prepare_process()`, `process()`, `_prefilter_chromaticity()` and `_guide_with_chromaticity()`, with the least-squares solve from `src/iop/choleski.h` (`pseudo_solve`), darktable's recursive Gaussian from `src/common/gaussian.c` and `gaussian.h` (`dt_gaussian_mean_blur`, `dt_gaussian_blur`, `dt_gaussian_blur_4c`, `_compute_gauss_params`), the bilinear resampling from `src/common/fast_guided_filter.h` (`interpolate_bilinear`), the gamut table and mapping from `src/common/darktable_ucs_22_helpers.h` (`dt_UCS_22_build_gamut_LUT`, `lookup_gamut`, `gamut_map_HSB`, `soft_clip`), darktable UCS from `src/common/colorspaces_inline_conversions.h`, and `scharr_gradient` and `deg2radf` from `src/common/math.h`. The OpenCL path is `process_cl` with `data/kernels/colorequal.cl`. The reference is darktable's CPU path.

Status: ported, in Slang (`crates/op/colorequal`), as an operation with its manifest, schema and `prepare` (`ColorEqual`): every parameter, with and without the guided filter; nothing is `NotPorted`. darktable's mask displays (its "show mask" quads) are its GUI's and stay behind. darktable's Gaussian and bilinear resampling are shared, in the colour library (`crates/color/slang/gaussian.slang`, `bilinear.slang`, and `src/gaussian.rs`, `bilinear.rs`), which the tone equalizer's EIGF is to use too, as is darktable UCS's gamut table (`apertrust_color::ucs::gamut_lut`), which colour balance rgb builds too (decision 65; Choices, 1). It is in `OPERATIONS` with `default_enabled` off (decision 28). Checked on one GPU so far.

Golden results (mire1.cr2, 3908×2602, Apple M4, a one-instance plan of the engine):

| Case | darktable test | ΔE2000 mean | p95 | max | over 2.3 | Module effect (mean, p95, max) |
| --- | --- | --- | --- | --- | --- | --- |
| `colorequal` | 0159-coloreq-guided-filter | 0.0000 | 0.0001 | 0.0006 | 0 | 8.149, 24.214, 46.723 |
| `colorequal-no-filter` | 0158-coloreq-no-guided-filter | 0.0001 | 0.0002 | 0.0347 | 0 | 11.325, 40.783, 66.836 |

## Purpose

The colour equalizer changes hue, saturation and brightness by hue, darktable's successor to color zones: eight nodes from red to magenta on each of three curves, so the photographer can turn greens towards yellow, deepen blues or brighten skin.

Algorithmically, each pixel's hue in darktable UCS reads three periodic curves, interpolated through their nodes by radial basis functions, giving a hue offset and gains on saturation and brightness; a guided filter, on by default, first smooths the chromaticity the hue is read from, then spreads the saturation and brightness corrections over areas of one colour, weighted by saturation so greys and soft edges stay put; the result is gamut-mapped and converted back. It is the Colour tab's Colour equalizer (`docs/modules/README.md`) in the first pass, on layers too.

## Pipeline

- **darktable position:** 41.2 in the v5.0 order (`v50_order`), after colorbalance (41.0), before color balance rgb (41.5); the same in v3.0, where tests 0158 and 0159 run it (their sidecars' `iop_order_version` 2). A scene-referred colour module before the tone mappers.
- **Apertrust slot:** working space 41.2, darktable's place and number: after rgb primaries (28.9), before colour balance rgb (41.5) (decision 22; `docs/engine.md`, Order).
- **Domain:** scene-referred. darktable's description: "linear, RGB, scene-referred" in, "quasi-linear, RGB" processing, "quasi-linear, RGB, scene-referred" out.
- **Input and output:** working RGB, linear Rec2020 (`default_colorspace` is `IOP_CS_RGB`). The kernel converts to XYZ D65 through the working profile and CAT16 (`XYZ_D50_to_D65_CAT16 · matrix_in`), then darktable UCS, and back (`matrix_out · XYZ_D65_to_D50_CAT16`), with `WorkingProfile::linear_rec2020`'s matrices. Alpha passes through.
- **Assumes earlier modules:** none; white balance and colour calibration make its hues meaningful.
- **Layer support:** yes. The whole-image passes see the operation's whole input, as darktable's module processes its whole region before blending; a layer's mask (Phase 4) blends the output.

## Operation

| Manifest field | Value |
| --- | --- |
| `id` | `colorequal` |
| `version` | 1 |
| `slot` | working space, 41.2 |
| `domain` | scene |
| `layers` | true |
| `differentiable` | true: `colorequal_backward` and `colorequal_forward_diff`, the derivatives of `main`, the pass that writes the output (Gradients) |
| `radius` | 0: the guided filter's passes see the whole image (`PassKind::Reduction`, decision 15's whole-image kind under its code name), and `main` reads only its own pixel and scratch |
| `default_enabled` | false (decision 28) |
| `provenance` | darktable, commit 20891f6c6f; the sources above |

`tiles_whole_image_passes` is true: the one pass that reads the input, `sample`, writes exactly the pixels of the tile it runs in, at their place in the whole image (Neighbourhood and scale).

## Parameters

darktable params version 4, 128 bytes: `threshold`, `smoothing_hue`, `contrast`, `white_level`, `chroma_size`, `param_size` (floats), `use_filter` (gboolean, 4 bytes), the 24 nodes (floats), `hue_shift` (float). The schema (version 1) has darktable's field names, in its order, with its introspection's ranges and defaults; `Settings::default` holds them, and a test checks the schema against it.

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| `threshold` | `threshold` | float | 0 to 0.3 | 0.1 | "saturation threshold": the saturation weights' inflection, `−0.015 + 0.3 · (5 · threshold)²`, 6% at the default | Colour equalizer: Saturation threshold |
| `smoothing_hue` | `smoothing_hue` | float | 0.05 to 2 | 1 | "hue curve": the hue curve's sharpness, its basis width `π / smoothing_hue` | Colour equalizer: Hue curve |
| `contrast` | `contrast` | float | −1 to 1 | 0 | "contrast": the saturation weights' steepness, `60 + 40 · contrast` | Colour equalizer: Contrast |
| `white_level` | `white_level` | float | −2 to 16 | 1 | "white level", in EV: the luminance `2^white_level` whose L* is UCS's white | Colour equalizer: White level |
| `chroma_size` | `chroma_size` | float | 1 to 10 | 1.5 | "hue analysis radius", pixels at full resolution: the prefilter's σ is half of it | Colour equalizer: Hue analysis radius |
| `param_size` | `param_size` | float | 1 to 128 | 1 | "effect radius", pixels at full resolution: the guide's σ is half of it | Colour equalizer: Effect radius |
| `use_filter` | `use_filter` | boolean | | true | "use guided filter" | Colour equalizer: Use guided filter |
| `sat_red` … `sat_magenta` | `sat_red` … | float | 0 to 2 | 1 | The saturation curve's eight nodes, gains (1 none) | Colour equalizer: Saturation tab |
| `hue_red` … `hue_magenta` | `hue_red` … | float | −180 to 180 | 0 | The hue curve's eight nodes, offsets in degrees | Colour equalizer: Hue tab |
| `bright_red` … `bright_magenta` | `bright_red` … | float | 0 to 2 | 1 | The brightness curve's eight nodes, gains (1 none) | Colour equalizer: Brightness tab |
| `hue_shift` | `hue_shift` | float | −23 to 23 | 0 | "node placement", degrees: every node's hue moved by it | Colour equalizer: Node placement |

The nodes sit at hues `k · 45° + hue_shift + 20°` in darktable UCS, red first (`ANGLE_SHIFT`: sRGB red lies at 20°).

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 | No `hue_shift`, 124 bytes; the threshold's inflection was 0.1 | `hue_shift` 0 |
| 2 | `hue_shift` | the threshold 0.024; every saturation and brightness node `1 + 0.75 · (v − 1)` (their sliders' range grew by 4/3); `hue_shift` not copied |
| 3 | | the threshold raised by 0.1; `contrast = −5 · max(0, threshold − 0.024)`; `hue_shift` not copied |
| 4 | current | |

Both tests are version 1. darktable's migrations from version 2 copy all but the struct's last field, so from version 2 on they leave `hue_shift` uninitialised, whatever `malloc` returns (a darktable bug): from version 1, `legacy_params`' first step writes 0 into a block the third step's `malloc` most likely reuses. `ports.rs` migrates version 1 with `hue_shift` 0, which the golden cases confirm (a mean of 0.0001), reads version 4 as it is, and refuses versions 2 and 3, which no case holds and whose migration is undefined.

## Setup on the CPU (`commit_params`, `_prepare_process`)

`prepare` (`crates/op/colorequal/src/setup.rs`) computes, with `s` darktable's `roi_in->scale / piece->iscale`, the pipe's scale, 1 in an export:

1. **The curves** (`commit_params`, `_periodic_RBF_interpolate`): for each of the three, a periodic interpolation through its eight nodes by radial basis functions, `φ(d) = exp(Σ_{l<m} exp(−l²/c) · cos(l · |d|))` with `m = ⌈3√c⌉` terms; the 8×8 matrix of `φ` between nodes is solved for the weights by `pseudo_solve` (the normal equations, Choleski-Banachiewicz, triangular descent and ascent, all f32; where it fails darktable leaves the nodes as the weights), and the table is the weighted bases at 512 hues from −π. `c` is π for saturation and brightness, `π / smoothing_hue` for hue, whose nodes are in radians. Saturation and brightness are clipped at 0. At the defaults the curves through eight 1s dip to 0.99854 between the nodes (Tool binding notes).
2. **The gamut table** (`dt_UCS_22_build_gamut_LUT`): the working gamut's boundary in darktable UCS, its colourfulness squared per hue, 512 entries, from the working space's RGB to XYZ D65; built once in `apertrust_color::ucs::gamut_lut`.
3. **The saturation weights** (`_init_satweights`): 8193 entries of `1 / (1 + exp((−60 − 40 · contrast) · i / 8192))` for `i` from −4096 to 4096, computed in double and stored as f32, as darktable's static table.
4. **The values** (`_prepare_process`): `white = Y_to_dt_UCS_L_star(2^white_level)`; `sat_shift = −0.015 + 0.3 · (5 · threshold)²`; `max_brightness` the largest brightness node, at least 1; `bright_shift = sat_shift + 0.01 · max_brightness · min(5, √param_size)`; `gradient_amp = 4 · √max_brightness · s²`; the σ of the prefilter `0.5 · chroma_size · s`, of the guide `0.5 · param_size · s`, of the saturation's blur and of its gradient's `max(0.5, s)`.
5. **The grids** (`_get_scaling`): each guided filter downsamples by `max(1, min(4, ⌊σ − 1.5⌋))`, its grid `(int)(width / scaling)` across and down, its Gaussians' σ `max(0.2, σ / scaling)` on it; darktable calls a grid smaller than the image `resized`.
6. **The feathering**: the prefilter's `10⁻⁵`, the guide's `10⁻⁶`.

## Per pixel and over the image (`process`)

With `in` the module's input, darktable's steps:

1. **STEP 1**: per pixel, `XYZ = M_in · rgb`, xyY (`dt_D65_XYZ_to_xyY`, negative XYZ clipped to 0), the chromaticity `UV = xyY_to_dt_UCS_UV`, `L* = Y_to_dt_UCS_L_star(Y)`, and the saturation `(max − min) / max` of the channels (0 where `max` or `max − min` is under `NORM_MIN`). Then the saturation blurred (`dt_gaussian_mean_blur`, σ `max(0.5, s)`).
2. **STEP 2, the prefilter** (`_prefilter_chromaticity`, with the guided filter): a guided filter of UV by itself on its grid: the covariance `[[UU, UV], [UV, VV]]` of the (downsampled) UV, both blurred, the covariance finished (`cov −= mean · mean`), `a = cov · (cov + ε I)⁻¹` (0 where the determinant is within `4 · FLT_EPSILON` of 0) and `b = mean − a · mean`, both blurred and upsampled, then each pixel's UV mixed towards `a · UV + b` by the weight of its blurred saturation less `sat_shift`. Where the grid is the image, darktable blurs UV in place, so the mix starts from the blurred UV.
3. **STEP 3**: per pixel, `JCH` from `L*`, `white` and the (prefiltered) UV (`dt_UCS_LUV_to_JCH`), HSB (`dt_UCS_JCH_to_HSB`), which darktable writes to its output buffer. With the filter, the saturation's gradient (`scharr_gradient` of the blurred saturation, clamped one pixel inside the image's edges), `gradient_amp · max(0, g − 0.02)²`, replaces L*. Where the chroma is above `NORM_MIN`, the corrections are the hue curve at the hue, the saturation curve at the hue, and the saturation times the brightness curve less 1; otherwise 0, 1 and 0.
4. **STEP 4, the guide** (`_guide_with_chromaticity`, with the filter): the saturation's gradient blurred (σ `max(0.5, s)`); on the guide's grid, the (downsampled) UV, the corrections and the brightness correction, the UV's covariance and its correlations with the saturation and brightness corrections, all blurred, the brightness correction at a tenth of the σ; the covariance and correlations finished, `a` and `b` as above with the correlations in place of the covariance, both blurred and upsampled; then per pixel the saturation correction `1 + (a₀ · UV + b₀ − 1) · w(sat − sat_shift)` and the brightness correction `(a₁ · UV + b₁) · (1 − clip(gradient)) · w(sat − bright_shift)`. Where the grid is the image, darktable blurs UV and both corrections in place, so the hue correction ends blurred too and the filter applies to the blurred UV.
5. **STEP 5**: per pixel, `H += hue correction`, `S = max(0, S · (1 + 2 · (saturation correction − 1)))`, `B = max(0, B · (1 + 8 · brightness correction))`, the saturation soft-clipped at constant brightness from 0.8 of the gamut boundary at that hue and lightness (`gamut_map_HSB`), then HSB to XYZ D65 (`dt_UCS_HSB_to_XYZ`) and back to working RGB.

**In the kernel** (`crates/op/colorequal/slang/colorequal.slang`): without the filter, `main` alone does steps 1, 3 and 5 per pixel. With it, `sample` writes each pixel's UV, saturation and lightness into scratch laid out as the whole image at the pipe's scale; the Gaussians run as line passes, one invocation per column then per row, each walking its line as darktable's loop does (the shared `gaussian.slang`); `main` recomputes each pixel's lightness from its input, bit for bit what `sample` wrote, reads its prefiltered UV and guided corrections from scratch, and does step 5.

## GPU plan

Every pass is `[numthreads(16, 16, 1)]`. With the filter, `prepare` gives, in order (the GPU plan for darktable's defaults on mire1 has 46 passes):

| Pass | Kind | Size | Reads | Writes |
| --- | --- | --- | --- | --- |
| `sample` | whole-image | the operation's input | the pixel | UV, saturation, L* at its place |
| `blur_columns1`, `blur_rows1` | whole-image | a line each | the saturation | the saturation, through the temporary |
| `prefilter_init` | whole-image | the prefilter's grid | UV | its grid's UV and covariance products |
| 3 × `blur_columns2`, `blur_rows2` | whole-image | a line each | the grid's UV, P1, P2 | the same, blurred |
| `prefilter_coefficients` | whole-image | the prefilter's grid | the grid | `a` (P1, P2), `b` (C1) |
| 3 × `blur_columns2`, `blur_rows2` | whole-image | a line each | `a`, `b` | the same, blurred |
| `prefilter_apply` | whole-image | the image | UV, the grid, the saturation, the weights' table | UV |
| `corrections_pass` | whole-image | the image | UV, L*, the saturation, the curves and weights | the corrections, the saturation's gradient (over L*), the two saturation weights |
| `blur_columns1`, `blur_rows1` | whole-image | a line each | the gradient | the gradient |
| `guide_downsample` (where the guide does not blur in place) | whole-image | the guide's grid | the corrections | their grid copy |
| `guide_init` | whole-image | the guide's grid | UV, the corrections | the grid's UV, covariance and correlations |
| 6 × `blur_columns2`, `blur_rows2`, then `blur_columns1`, `blur_rows1` | whole-image | a line each | the grid's buffers | the same, the brightness correction at σ / 10 |
| `guide_coefficients` | whole-image | the guide's grid | the grid | `a`, `b` |
| 3 × `blur_columns2`, `blur_rows2` | whole-image | a line each | `a`, `b` | the same |
| `guide_apply` | whole-image | the image | the UV it applies to, `a`, `b`, the weights, the gradient | the saturation and brightness corrections |
| `main` | per pixel | the image | the pixel, UV and the corrections at its place | the pixel |

One run (decision 15): `sample` reads the input, every later whole-image pass only scratch. The scratch: the image's UV, corrections and weights (float2), saturation, L* and brightness correction (float), a temporary for the Gaussians' columns, and the grids' buffers, `GRID_UV`, `P1`, `P2`, `C1`, `C2`, each float2, the 4×2 covariance, correlations and coefficients in float2 halves as the line passes need (decision 15), plus the guide's corrections on its grid where it does not blur in place. 84 bytes a pixel where both filters blur in place (darktable's defaults), about 77 with test 0159's downsampled guide: 0.85 GB for mire1, 5.1 GB at 61 MP, beside the operation's input and output. Each pass binds at most eight storage buffers (`setup.rs`'s test). The passes' uniforms hold only what they read (decision 15): the prefilter's neither the curves nor the effect radius (a unit test); every parameter but the node placement's effect through the curves feeds some whole-image pass, so a change of any reruns the filter, and only `main`'s gradient slots' values are its own.

Without the filter, one per-pixel pass, `main`, binding three 16-byte placeholders where the guided `main` reads its scratch, so both end in the same entry point with the same bindings and the derivative entry points serve both, as local contrast's two modes do.

## Neighbourhood and scale

- **Neighbourhood radius or reach** (decision 13): the Gaussians are recursive, so every pixel depends on its whole row and column, and the guide's on the whole grid. In practice the whole image: the guided filter is whole-image passes, and the reach is 0 (`Operation::reach`'s default from the manifest's radius 0). darktable's `tiling_callback` asks for an overlap of `16 + max(chroma_size, param_size)` ("safe feathering"), so darktable's tiled result differs from its untiled one near tile edges; it tiles only when memory runs short, and the golden renders do not tile.
- **Scale dependence:** darktable scales every σ by `s` (`hue_sigma`, `par_sigma`, and `max(0.5, s)` for the saturation and its gradient), `gradient_amp` by `s²`, and derives each grid's downsampling from its scaled σ. Apertrust follows the scaled σ at every pipe's scale, unrounded, as darktable does: no rounding to whole pixels enters until the grid's downsampling, which keeps darktable's (decision 15).
- **What Apertrust computes at each scale** (decision 15; principle 2): the guided filter over the whole image at the pipe's scale, in every pipe. One choice is not darktable's at a smaller scale: **whether a filter blurs in place.** darktable blurs the image's own UV and corrections in place where a grid is not downsampled, which the code makes a behaviour: the prefilter then mixes towards the blurred UV, and the guide blurs the hue correction and applies to the blurred UV. Its export takes that path only for a σ under 3.5 pixels at full resolution, but its preview, with σ scaled down, takes it at larger radii too: test 0159's effect radius of 25, downsampled by 4 in the export, is in place below a scale of about 0.28, where the hue correction, ±45° on that test, becomes a blur of itself. Apertrust takes the in-place choice from the export's σ, at full resolution, and the grid's geometry from the pipe's (`Grid::scaled`), so a view at 1/4 or 1/8 does the export's processing at its scale; at scale 1 it is darktable's exactly. This is decision 15 (3)'s rule for every port: a branch darktable takes on a length scaled to the pipe follows the full-resolution length, as darktable-cli's export does at any size, while a size derived from the scaled length keeps darktable's derivation at the pipe's scale. On test 0159 the viewport's gap is a mean of 0.81 ΔE2000 at 1/4 and 0.62 at 1/8 (Results), where darktable's preview's choice would leave 1.86 and 1.32.
- **Tile overlap needed:** none for `main`.
- **Tiling in Apertrust:** the colour equalizer opts in to a first round of tiles (`Operation::tiles_whole_image_passes`, decision 15). `sample`, the one pass that reads the input, writes the tile's own pixels at their place in the whole image; every filter pass then runs once over the whole scratch, and the second round's `main` reads it. Without the filter, there are no whole-image passes and `main` tiles as any per-pixel pass. `a_tiled_export_with_the_colour_equalizer_equals_the_whole_one` (`crates/engine/tests/tiles.rs`) holds a tiled export bit-identical to the whole one, in two rounds, at darktable's defaults and with test 0159's radii and curves.
- **lavapipe's 128 MiB binding:** every image-shaped scratch buffer binds whole, the float2 ones at 8 bytes a pixel, so above about 16.8 MP (2²⁴ pixels) no tile plan fits lavapipe's binding, and an export with the guided filter is refused (`auto_tile`, `ExportError::NoTiles`), as bloom's is above 33.5 MP; so is the 100% view above 16.8 MP. Decision 15's strips cannot lift it alone: the per-pixel passes over scratch could bind windows, but a column's recursion walks the whole column in darktable's order. The refusal is accepted, an exception on decision 7's list (decision 15). The remedy is decision 7's transposed line passes after a block transpose, with darktable's sums unchanged.
- **Region-of-interest changes:** none (darktable's defaults).

## Precision and special values

- **fp64 in darktable:** only the saturation weights' table, computed in double and stored as f32; `prepare` computes it in f64 too. Everything else is f32 in darktable and in the port.
- **`-ffast-math`:** darktable's release builds compile with it (`colorequal.c` itself leaves `extra_optimizations.h` out, "results in crashes"), which may reorder the solve's sums and the bilinear's divisions, and makes `scharr_gradient`'s `dt_fast_hypotf` `√(x² + y²)`, which the kernel computes. Nothing magnifies these last bits; the cases' means are 0.0000 and 0.0001.
- **The curves' solve** runs in Rust's f32 in darktable's order; Rust's `exp` and `cos` may differ from the C library's by an ulp, which the tables carry into values well under the cases' rounding.
- **Divisions:** UCS's divisions are the colour library's, guarded as darktable's (`FLT_MIN`); the white's L* is above 0 for any white level; `soft_clip` at a zero-width boundary returns the boundary, as darktable's arithmetic does through an infinity; the 2×2 inverse is 0 where darktable's determinant test fails, as darktable sets it.
- **NaN and infinity:** darktable checks for neither. The Gaussian clamps every value it reads to ±10⁹, darktable's `dt_gaussian_mean_blur` bounds, so an unbounded value cannot run along a line. **A guard, not darktable's: each input channel is bounded at ±10³⁰** (`MAX_INPUT` in `sample` and `main`), so no infinite pixel reaches the colour maths; it changes no finite value a photo has. Output stays finite for black, 10⁻³⁰, negative, 10⁶ and saturated input at every parameter's ends (a GPU test).
- **Small images:** darktable's gradient reads outside an image under three pixels across or down, and its grid of `(int)(width / scaling)` can be 0 wide; the kernel clamps the gradient's neighbours to the image, and the grid is at least one pixel, neither of which changes an image of three pixels or more.

## CPU and OpenCL differences in darktable

- **The saturation's gradient's square**: OpenCL's `process_data` computes `gradient_amp · k · k`, the CPU `gradient_amp · (k · k)`, a last bit; the port follows the CPU.
- **The Gaussian** on OpenCL is `dt_gaussian_blur_cl_buffer`, the same recursion in another order of work; not compared.
- Everything else, the in-place blurs included, is the same on both paths. The port follows the CPU path; the references are rendered on the CPU.

## Deliberate differences

- **The in-place blurs at smaller scales follow the export's σ** (Neighbourhood and scale): principle 2's same processing at each scale, not a departure from darktable's export, where the scale is 1.
- **`lookup_gamut`'s branch for a hue on an entry is gone**, as in colour balance rgb: its weight is 0 there, so no value changes, and Slang's backward pass would recompute it.
- **`main` reads the three curves for every pixel and then chooses**, where darktable reads them only above the neutral threshold (`C > NORM_MIN`): a branch around the reads would let Slang's backward pass, recomputing the chroma, read table entries the forward pass never saved (the `slang-kernels` trap). The outputs of both cases are bit-identical to the branching kernel's.
- **The colour library's JCH to xyY takes its white as a constant**, so `main` passes J and C already multiplied by the white and a white of 1: darktable's own products, `J · L_white` and `C · L_white`, so no value changes, and the white level's derivative reaches them.
- **Version 1's migrated `hue_shift` is 0** where darktable leaves it uninitialised (Parameter versions).
- **Guards:** inputs bounded at ±10³⁰; the gradient's neighbours and the grids' sizes for images under three pixels (Precision). None changes an image a photo makes.
- **The gradient slots' derivative tables** (Gradients), which change no value.

## Choices

1. **darktable's Gaussian, bilinear resampling and UCS gamut table in the colour library** (decision 65): the colour equalizer is the second port to call `dt_gaussian_blur` and `interpolate_bilinear`, after the tone equalizer's EIGF, and the second to build `dt_UCS_22_build_gamut_LUT`, after colour balance rgb. Decision 65 rules out copies once a second user exists, so the earlier copies move onto the shared ones, with the same names, in planned refactoring that leaves the golden cases bit for bit unchanged:
   - the EIGF's (`crates/op/toneequal/slang/filters/eigf.slang`, `src/filters.rs`, its `Deriche`): its generated WGSL changes in names, its functions the `gaussian` and `bilinear` modules' rather than `eigf`'s, though not in arithmetic;
   - colour balance rgb's `ucs_gamut_lut`, to call `apertrust_color::ucs::gamut_lut`: on the CPU, changing no kernel.
2. **The guided filters' 2×2 matrices in float2 halves**, P1 and P2, C1 and C2: the line passes walk whole lines, so each buffer must fit one binding (decision 15); darktable's covariance holds `UV` twice, which the halves keep, so the arithmetic is darktable's.
3. **Pass bindings within eight storage buffers.** The guide's apply would need nine, so `corrections_pass` writes the two saturation weights per pixel (`w(sat − sat_shift)` and `w(sat − bright_shift)`), which `guide_apply` reads; they are darktable's same products in the same order.
4. **`main` recomputes each pixel's L* and reads the prefiltered UV**, where darktable keeps HSB in its output buffer: the output is not a whole-image pass's to write (decision 15), and L* and UV take 8 bytes a pixel against HSB's 12. Where darktable's guide blurs UV in place, the port blurs a copy (`GRID_UV`), so `main`'s UV is the prefiltered one darktable computed HSB from.
5. **Both modes end in `main`** with the same bindings, so the derivative entry points serve both; without the filter its three placeholders keep Phase 2's fuser, which fuses only passes binding nothing but input, output, uniforms and tables, from fusing it. When the fuser is built, each mode gets its own bindings, with no placeholders. A fuser that lets derivative entry points follow each output pass stays in the plan for later.
6. **One buffer per line pass.** Several per pass would share the walks' latency (Results); kept simple for now. Planned with Phase 3's performance work: several buffers per line pass in the prefilter's six full-resolution blurs, which take most of a render, measured before and after on the M4 and a software GPU, the output bit for bit the same.

## Tool binding notes

In the first pass (decision 2) the Colour equalizer module is darktable's and owns every parameter (`docs/modules/README.md`): content tabs Hue, Saturation and Brightness, each its eight nodes (`hue_<c>` in degrees; `sat_<c>` and `bright_<c>` shown as percentages less 100, so 1 is 0 %), darktable's curve graph a lossless view of them, Node placement (`hue_shift`, in degrees, with its picker) under it on every tab; the Options group: White level (`white_level`, EV, soft range −2 to +2, with its picker), Hue curve (`smoothing_hue`, shown on the Hue tab only), Use guided filter (`use_filter`), Hue analysis radius (`chroma_size`, px), Saturation threshold (`threshold`, %, its show-mask quad), Contrast (`contrast`) and Effect radius (`param_size`, px, its show-mask quad), the last four shown only with the guided filter. darktable's presets, all version 4 and with the guided filter: bleach bypass, Kodachrome 64 like (effect radius 19), Kodak Portra 400 like (node placement −23°), teal & orange (hue curve 0.52).

- **What is an identity:** none. At darktable's defaults the guided filter's prefilter smooths the chromaticity the output is rebuilt from, wherever the saturation is past the threshold (up to half a pixel's largest channel on a test image of patches); without the filter the curves through nodes at 1 still dip to 0.99854 between the nodes, the gamut mapping compresses every saturation past 0.8 of the boundary, and negative XYZ is clipped (`no_setting_is_an_identity`). So the Colour equalizer module is switched (decision 32): untouched while off, its controls at darktable's defaults, any change turning it on with them and its reset turning it off.
- **Derivatives:** the hue, saturation and brightness curves' red nodes and the white level are the gradient slots (Gradients).

## Gradients

The derivative entry points are those of `main`, the pass that writes the output, over the scratch the forward render left (decision 15), against four slots, in this order: `hue_red` (degrees), `sat_red`, `bright_red`, `white_level` (EV). The curves are linear in their nodes, so a curve's derivative with respect to its red node is the curve through a 1 at red and 0 elsewhere, per degree for the hue's, and 0 where the saturation or brightness curve is clipped; `prepare` tabulates it beside the curves, and `main`'s per-pixel function adds `table · (slot − value)`, which is 0 in value and carries the derivative, as RGB curve substitutes its slots. The white level goes through `Y_to_dt_UCS_L_star(2^white_level)` in the kernel.

- **Without the guided filter** every slot is exact: `main` is the operation. The case names the three node slots (`ports.rs`) at a step of 1e-2: at 1e-3 the hue node moves each pixel by about 1e-7 of its value, f32 rounding of two renders. The white level, exact too, is not named: in exact arithmetic the white cancels in UCS's round trip and acts only through the corrections and the gamut mapping, so its total on test 0158 (160 for 10 MP) is what is left of per-pixel terms of both signs, and darktable's neutral threshold (chroma above `NORM_MIN`, which the white divides) switches near-grey pixels' corrections as it moves, which finite differences see and no derivative does: 2% to 70% off at steps from 1e-3 to 0.2, while forward and reverse mode agree within 3.8e-5. It passes on the synthetic sweep at 1e-2, and `the_slots_derivatives_are_the_whole_renders_without_the_filter` checks all four on an image away from that threshold.
- **With the guided filter** every slot feeds a whole-image pass as well as `main` (the corrections the guide spreads, and the white through the saturation the brightness correction takes), so none is exact and the case names none (decision 11). The derivative entry points then give `main`'s share with the guided corrections held: 0 for the nodes.

Results (Apple M4, the sum of output RGB as the loss, step 1e-2):

| Parameter | Case `colorequal-no-filter`: relative error | Forward vs reverse | Synthetic: relative error | Forward vs reverse |
| --- | --- | --- | --- | --- |
| `hue_red` | 2.6e-3 | 1.1e-6 | 1.4e-3 | 1.2e-6 |
| `sat_red` | 4.5e-4 | 1.8e-6 | 1.1e-4 | 1.4e-6 |
| `bright_red` | 1.8e-5 | 1.4e-7 | 2.3e-4 | 1.8e-7 |
| `white_level` (not named; measured by hand) | 0.20 | 3.8e-5 | 1.4e-3 | 4.9e-6 |

On the GPU test's saturated image (64×48, step 1e-2), every slot's reverse mode is within 1% of finite differences and within 1e-4 of forward mode.

Known non-smooth spots: the neutral threshold (`C > NORM_MIN`), the clipping of saturation, brightness and the saturation and brightness curves at 0, the gamut's soft clip's start at 0.8 of the boundary, and, with the filter, the 2×2 inverse's determinant test.

## Results

**Golden** (Apple M4, a one-instance plan of the engine): the table at the top.

**Speed** (Apple M4, a render of the plan, every pass, GPU timestamps, 3908×2602): with the guided filter on test 0159, 175 to 189 ms a render, the medians of three runs (minimums 150 to 155 ms), of which the line passes take about 100 ms (the 15 float2 column passes about 43 ms and row passes 60 ms, most of them the prefilter's six blurs at full resolution), `sample` 5 to 7 ms and each per-pixel pass over the image 6 to 10 ms. The line passes are latency-bound: 3908 or 2602 invocations, each walking its line's recursion in order. Without the filter, test 0158: 4 to 8 ms a render, one per-pixel pass.

**On the GPU, needing no darktable** (`crates/golden/tests/colorequal.rs`): the operation equals darktable's `process()` restated on the CPU (`apertrust_op_colorequal::reference`, loop by loop with its buffers blurred in place as darktable's are) within 7.3e-6 of each pixel's largest channel on a 64×60 image of every hue at five saturations and four values, without the filter, at darktable's defaults (both grids the image), at test 0159's radii (the guide's grid a fourth) and with both grids a third; alpha passes through. At a fourth of a 256×240 image, with test 0159's effect radius and a hue analysis radius of 10, whose export downsamples by 4 and 3 while the view's grids are the image, it equals the restatement whose filters blur copies as the export's do within 6.5e-6, and is 0.22 from darktable's preview, which blurs in place (`a_smaller_view_does_the_exports_processing_at_its_scale`).

**The viewport against the export** (decision 15's target, `the_viewport_is_within_decision_80s_target_of_the_export`, ignored, run by hand): on test 0159's input, the operation on the fitted image against the export downscaled by the viewport's box filter, mean ΔE2000. The colour equalizer has a version without its filtering, darktable's own mode without the guided filter, so the baseline is decision 15's own, as the tone equalizer's unfiltered mask: the same gap with the filter off. Decision 15 (4)'s baseline, the last step with the export's whole-image results, is printed too, but here it also removes the curves' own per-pixel gap, since the corrections, a per-pixel lookup of the curves, come out of a whole-image pass.

The baseline that counts is decision 15 (2)'s: the per-pixel gap stated per step, every value a pixel reads from other pixels held at the export's (the saturation's and the gradient's blurs, the prefilter's and the guide's coefficients, whatever a filter blurs in place) and every value from the pixel's own and those at the viewport's scale (`sample`, the prefilter's apply, the curves' lookups and the saturation weights, the guide's apply and `main`), through a setting only the tests reach. It is to be measured on mire1 and decision 15's four corpus renders, at darktable's defaults and test 0159's settings, before the Colour equalizer module ships; until then the gap on test 0159's input, below, is the regression bound.

| Scale | Viewport against the export | Baseline: without the filter | Filter's share | Target | With the export's whole-image results |
| --- | --- | --- | --- | --- | --- |
| 2400×1598 | 0.316 (p95 1.01, max 13.0) | 0.392 | −0.077 | 0.15 | 0.048 |
| 1/2, 1954×1301 | 0.366 (p95 1.12, max 10.9) | 0.558 | −0.193 | 0.15 | 0.070 |
| 1/4, 976×650 | 0.812 (p95 2.40, max 18.2) | 0.891 | −0.079 | 0.4 | 0.209 |
| 1/8, 488×325 | 0.615 (p95 1.86, max 18.4) | 1.102 | −0.486 | 0.4 | 0.437 |

The filter narrows the gap at every scale: the curves' large hue offsets (+43° red, −101° orange at the second entry of test 0158, ±45° on 0159) make the operation strongly nonlinear in the chromaticity of a pixel the viewport averages, and the filter smooths the chromaticity first. Against decision 15 (4)'s baseline the share would be 0.27, 0.30, 0.60 and 0.18, over the target at three scales, which counts that nonlinearity as the filter's.

## Test plan

- **Golden cases:** `colorequal`, darktable's test 0159-coloreq-guided-filter, and `colorequal-no-filter`, test 0158-coloreq-no-guided-filter (the second of its two entries), both on mire1.cr2, unchanged, version 1 migrated, sigmoid off after them. `Tolerance::DARKTABLE`, one-instance plans of the engine (decision 15).
- **Unit tests** (`crates/op/colorequal`): the schema's defaults are darktable's, in darktable's order, and every parameter reaches its setting and round-trips; the default curves are flat but for the ripple (1.5e-3), hue nodes at 0 give 0 exactly; a curve passes through its nodes; the curves are linear in their nodes, so the derivative tables equal finite differences of the curves; the solve solves and fails on a singular matrix; the weights are darktable's logistic; the process values are darktable's at scale 1 and 1/2; the grids downsample as darktable (test 0159's 977×650 at σ 3.125); a smaller view blurs in place only where the export does (`Grid::scaled`: test 0159's σ 12.5 a fourth and not in place at scale 1, the image and still not in place at 0.25; a σ under 3.5 in place at every scale, 3.5 itself not); without the filter one pass; with it every pass within eight storage buffers, binding no buffer twice, only `main` per pixel and writing the output, only it and `sample` reading the input, `main` binding the same in both modes; the prefilter's passes hold neither the effect radius nor the curves; `prepare` follows the pipe's scale and lays a tile's scratch out as the whole image.
- **The shared helpers** (`crates/color`): the Gaussian's coefficients have unit gain, a constant image stays constant, an impulse spreads symmetrically with unit mass, values are clamped as darktable's `CLAMPF` does; the bilinear resampling decimates a downscale by four and interpolates and clamps an upscale.
- **In `ports.rs`:** tests 0158 and 0159 read as version 1 with darktable's migration; 0158 names the three node slots and 0159 none; each gradient parameter moves its own setting; versions 2 and 3 are refused.
- **On the GPU** (`crates/golden/tests/colorequal.rs`, run in CI): the operation against its restatement on the CPU in four settings, and at a fourth against the restatement with the export's in-place choice (Results); no setting is an identity, with the filter or without; output stays finite for extreme input at every parameter's ends; the four slots' reverse mode against finite differences of whole renders and against forward mode, without the filter. And, ignored, run by hand: the viewport's target (Results). In `crates/engine/tests/tiles.rs`, run in CI: a tiled export through the default pipeline and the colour equalizer changes the image and equals the whole one bit for bit at every tile size, in two rounds.
