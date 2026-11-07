# Port spec: local contrast (bilat)

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/bilat.c` `commit_params()` and `process()`, with the local laplacian filter from `src/common/locallaplacian.c` and `locallaplacian.h` (`local_laplacian_internal`), the bilateral grid from `src/common/bilateral.c` (`dt_bilateral_grid_size`, `_splat`, `_blur`, `_slice`), `dt_fast_expf` from `src/common/math.h`, and the working space's conversions around a Lab module from `src/common/iop_profile.c` and `src/common/colorspaces_inline_conversions.h`. The OpenCL paths are `src/common/locallaplaciancl.c` with `data/kernels/locallaplacian.cl`, and `src/common/bilateralcl.c` with `data/kernels/bilateral.cl`. The reference is darktable's CPU path.

Status: ported, both modes, in Slang (`crates/op/bilat`), as an operation with its manifest, schema and `prepare` (`Bilat`): the local laplacian filter, darktable's default for a new instance (decision 22), and the bilateral grid, its other mode. It is in `OPERATIONS` with `default_enabled` off, as darktable has local contrast off until the user turns it on (decision 28). Checked on one GPU so far.

Golden results (mire1.cr2, 3908×2602, Apple M4), each a one-instance plan of the engine (decision 15):

| Case | darktable test | Mode | ΔE2000 mean | p95 | max | Pixels over 2.3 | Render (median, min) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `bilat` | 0002-local-contrast, params version 3, darktable's defaults | local laplacian filter | 0.0000 | 0.0001 | 0.0002 | 0 | 58 to 63 ms (41 to 47) |
| `bilat-bilateral` | 0002, edited: the bilateral grid, coarseness 50, contrast 20, detail 200 % (1.0) | bilateral grid | 0.0000 | 0.0001 | 0.0002 | 0 | 12 to 14 ms (8 to 11) |

The module's effect, darktable's input against its output: `bilat` a mean ΔE2000 of 0.856, p95 2.50, max 9.06; `bilat-bilateral` 2.42, p95 6.62, max 15.8. The port reproduces darktable to the f32 rounding of its sums: the fast exponential bit for bit, the pyramid's and the grid's geometry exactly (the grid's against darktable's own `-d dev` line).

## Purpose

Local contrast for the photographer: more or less detail and clarity without moving the image's overall tones. In the local laplacian mode it also controls contrast in the shadows and in the highlights separately, down to a local tone mapping that compresses a high dynamic range (darktable's "HDR local tone-mapping" preset).

Algorithmically, by mode:

- **Local laplacian filter** (Paris, Hasinoff and Kautz's, as darktable implements it with Aubry's fast approximation): the lightness of CIE Lab is remapped by a curve around each of six grey levels, and a Laplacian pyramid assembled from the six remapped images, each level's coefficient interpolated by the input's local lightness at that level.
- **Bilateral grid:** the lightness is splatted into a bilateral grid, blurred spatially and differentiated along lightness, and the result sliced back and added to each pixel's lightness, scaled by the detail.

darktable's module is "local contrast" (alias "clarity"); in Apertrust it is the Base tab's Local contrast (`docs/modules/README.md`).

## Pipeline

- **darktable position:** 54.0 in the v5.0 order, after relight (53) and before colorcorrection (55), in darktable's display-referred group after the tone mappers.
- **Apertrust slot:** display 54.0, darktable's place and number: after AgX (working space 45.5), before grain (display 65) (decision 22; `docs/engine.md`, Order).
- **Domain:** display-referred. darktable's description: "non-linear, Lab, display-referred" in, out and in processing. Test 0002 has no tone mapper, so there it runs on scene-referred values, Lab lightness above 100 included.
- **Input and output:** darktable's bilat is a Lab module (`default_colorspace` returns `IOP_CS_LAB`): its pipe converts the working space to Lab before it, with the working profile's matrix to XYZ D50 and `dt_XYZ_to_Lab`, and back after it. Apertrust's pipeline stays in working RGB, linear Rec2020, so the kernel makes both conversions itself, with the same matrices (`WorkingProfile::linear_rec2020`) and darktable's Lab formulas (`xyz_to_lab_d50` and `lab_to_xyz_d50` in the colour library), as grain does. Only L changes; a and b stay, and alpha passes through.
- **Assumes earlier modules:** demosaic and the input profile. Lightness only, so neither white balance nor the tone mapper is assumed.
- **Layer support:** yes. The whole-image passes see the operation's whole input, as darktable's module processes its whole region before blending; a layer's mask (Phase 4) blends the output.

## Operation

| Manifest field | Value |
| --- | --- |
| `id` | `bilat` |
| `version` | 1 |
| `slot` | display, 54.0 |
| `domain` | display |
| `layers` | true |
| `differentiable` | true: `bilat_backward` and `bilat_forward_diff`, the derivatives of `main`, the pass that writes the output (Gradients) |
| `radius` | 0: the whole-image passes see the whole image (`PassKind::Reduction`, decision 15's whole-image kind under its code name), and `main` reads only its own pixel and scratch |
| `default_enabled` | false (decision 28) |
| `provenance` | darktable, commit 20891f6c6f; `src/iop/bilat.c`, `src/common/locallaplacian.c`, `src/common/bilateral.c`, `src/common/math.h`, `src/common/iop_profile.c`, `src/common/colorspaces_inline_conversions.h` |

`prepare` gives the passes of the mode (GPU plan) with their uniforms and scratch buffers. Both modes end in the same entry point, `main`, binding the same resources, its uniform's mode choosing the path, as colour balance rgb's formula does; so the derivative entry points serve both.

## Parameters

darktable params version 3, 20 bytes: `mode` (an enum, 4 bytes), `sigma_r`, `sigma_s`, `detail`, `midtone` (floats). The schema (version 1) has darktable's field names, its introspection ranges and defaults, the local laplacian filter by default (decision 22); `Settings::default` holds them, and a test checks the schema against it.

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| `mode` | `mode` | choice: `bilateral_grid`, `local_laplacian` | | `local_laplacian` | The filter: darktable's "bilateral grid" (0) or "local laplacian filter" (1) | Local contrast: Mode |
| `detail` | `detail` | float | −1 to 4 | 0.25 | Local contrast added (both modes): the clarity term's weight, or the bilateral grid's slice gain; darktable shows it as 100 % more (offset 100), so 0.25 is its 125 % | Local contrast: Detail |
| `sigma_r` | `sigma_r` | float | 0 to 100 | 0.5 | Local laplacian: highlights, the slope of the curve below each grey level beyond the mid-tone range (1 keeps it, 0 flattens it); bilateral grid: contrast, the range sigma in L | Local contrast: Highlights; Contrast |
| `sigma_s` | `sigma_s` | float | 0 to 100 | 0.5 | Local laplacian: shadows, the slope above each grey level; bilateral grid: coarseness, the spatial sigma in pixels at full resolution | Local contrast: Shadows; Coarseness |
| `midtone` | `midtone` | float | 0.001 to 1 | 0.5 | Local laplacian: the mid-tone range σ, in lightness / 100, the half-width of the curve's middle and of the clarity term | Local contrast: Midtone range |

darktable's sliders limit them more than the parameters do: highlights and shadows to 0 to 2 (their hard maximum, shown as percentages), coarseness from 3 and contrast from 1 (their hard minimums, the bilateral grid's sliders defaulting to 50 and 20 when the mode is chosen, `gui_update`), the mid-tone range with three digits. The schema's ranges are the parameters', a control's span is the binding's.

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 | `sigma_r`, `sigma_s`, `detail`, 12 bytes | `midtone` 0.2, the bilateral grid |
| 2 | Prepends `mode` (`uint32_t`) | `midtone` 0.2 |
| 3 | Appends `midtone` | Current |

Test 0002 is version 3. `ports.rs` migrates versions 1 and 2 as darktable does, a unit test covering each; no case contains them.

## Setup on the CPU (`commit_params`)

darktable copies the parameters (`*d = *p`) and does the rest in `process`, from the region's size and scale; `prepare` does it from the input's size and the pipe's scale (`ImageContext::scale`), in f32 as darktable does:

- **Local laplacian** (`local_laplacian_internal`): `num_levels = min(30, ⌊log₂ min(wd, ht)⌋)` (`31 − clz`, the index of the shorter side's highest bit), `last = num_levels − 1`, the padding `max_supp = 2^last`, the padded level 0 `w₀ = wd + 2·max_supp` by `h₀ = ht + 2·max_supp`, and level `l`'s size `dl(w₀, l)`, halving as `(w − 1)/2 + 1`. The six grey levels `γₖ = (k + 0.5)/6`. mire1 at full resolution: 11 levels, `max_supp` 1024, level 0 5956×4650, level 1 2978×2325.
- **Bilateral grid** (`dt_bilateral_grid_size`, `L_range` 100): `σs = sigma_s · scale`, the pipe's scale for darktable's `1 / max(iscale / roi_in→scale, 1)`, raised to 0.5; `x = clamp(round(wd/σs), 4, 3000)`, likewise `y`; `z = clamp(round(100/sigma_r), 4, 50)`; the grid's own `σs' = max(ht/y, wd/x)` and `σr' = 100/z`, their reciprocals, and the grid `⌈wd/σs'⌉ + 1` by `⌈ht/σs'⌉ + 1` by `⌈100/σr'⌉ + 1` nodes, each product of a size and a reciprocal in f32 as darktable rounds it. Test 0002's edited case gets darktable's own grid, 79×53×6 with σs' 50.102566 (darktable's `-d dev` line), which a unit test holds. The slice's `norm = −detail · σr' · 0.04` is the kernel's, since `detail` is differentiated.

## Local laplacian filter (`local_laplacian_internal`)

On `v = L / 100` (`ll_pad_input`), a grey image padded by `max_supp` on every side by replicating its edges:

1. **The input's Gaussian pyramid** `G₀ = v` padded, `G_l = reduce(G_{l−1})` up to `G_last`, which darktable writes straight into the output pyramid (`gauss_reduce`): each coarse pixel `(i, j)` with `1 ≤ i ≤ w_l − 2` (rows likewise) is the 5×5 binomial `[1 4 6 4 1]² / 256` of the finer level centred on `(2i, 2j)`, and the coarse level's border pixels copy their inner neighbours (`ll_fill_boundary1`).
2. **Six remapped pyramids:** `Bₖ,₀ = curve(G₀, γₖ)` (`apply_curve`), `Bₖ,l = reduce(Bₖ,l−1)`. The curve (`curve_scalar`), with `c = x − g`, σ the mid-tone range, `s` shadows (`sigma_s`), `h` highlights (`sigma_r`), `d` the detail:
   - `c > 2σ`: `g + σ + s (c − σ)`; `c < −2σ`: `g − σ + h (c + σ)`;
   - `0 < c ≤ 2σ`: `t = c / 2σ`, `g + 2σ(1 − t)t + t²(σ + σs)`; `−2σ ≤ c ≤ 0`: `t = −c / 2σ`, `g − 2σ(1 − t)t + t²(−σ − σh)`, quadratic Béziers that join the lines with the same slope;
   - plus the clarity term `d · c · fast_exp(−c² / (2σ²/3))`, with darktable's `dt_fast_expf`: the float whose bits are `max(0, ⌊0x3f800000 + x · (0x402DF854 − 0x3f800000)⌋)`, a bit-pattern approximation of eˣ within 6 % on [−100, 0].
3. **Assembly, coarse to fine:** `O_last = G_last`; for `l = last − 1` down to 0, `O_l = expand(O_{l+1})` (`gauss_expand`: the coarse level's 3×3, 2×3, 3×2 or 2×2 stencil by the parities of `i` and `j`, darktable's `ll_expand_gaussian`, computed for `1 ≤ i ≤ ((w_l − 1) & ~1) − 1` and the border copied, `ll_fill_boundary2`), plus, at every pixel, the Laplacian coefficients of the two remapped pyramids whose grey levels bracket `G_l(i, j)`: `hi` the first `k ≥ 1` with `γₖ > G_l`, at most 5, `lo = hi − 1`, `a = clamp((G_l − γ_lo)/(γ_hi − γ_lo), 0, 1)`, and `O_l += (1 − a)·(B_lo,l − expand(B_lo,l+1)) + a·(B_hi,l − expand(B_hi,l+1))`, each expansion at the pixel clamped into the computed range (`ll_laplacian`).
4. **Write back:** `L = 100 · O₀` at the image's pixels, `max_supp` in from the padded edges; a and b copied.

At `sigma_r = sigma_s = 1` and `detail = 0` the curve is the identity, every remapped pyramid is the input's, and the assembly telescopes to `G₀`: the filter is an identity up to rounding. At darktable's defaults (shadows and highlights 0.5) it compresses contrast more than 2σ from each grey level, even at detail 0.

## Bilateral grid (`dt_bilateral_*`)

1. **Splat** (`dt_bilateral_splat`): each pixel `(i, j)` with lightness L goes to grid position `x = clamp(i/σs', 0, size_x − 1)`, `y` likewise, `z = clamp(L/σr', 0, size_z − 1)`, its cell `(min(⌊x⌋, size_x − 2), …)` and fractions; it adds `100/σs'²` times its trilinear weights to the cell's eight nodes. darktable splats per thread into slices of rows and merges them; the grid is their sum.
2. **Blur** (`dt_bilateral_blur`): along x, then y, the 5-tap `[1 4 6 4 1] / 16` with zero outside the grid (`blur_line`); then along z, the derivative filter `out[z] = 4/16 (in[z+1] − in[z−1]) + 2/16 (in[z+2] − in[z−2])`, zero outside (`blur_line_z`).
3. **Slice** (`dt_bilateral_slice`): at each pixel, the trilinear interpolation of the grid at its position, and `L' = max(0, L + norm · value)` with `norm = −detail · σr' · 0.04`; a and b copied.

At detail 0 the slice adds nothing: `L' = max(0, L)`, an identity on every pixel whose lightness is not negative.

## GPU plan

Every pass is one invocation per pixel of what it writes, `[numthreads(16, 16, 1)]`; the image's own passes index the whole image at the pipe's scale (an export runs the operation whole, with everything before it: Neighbourhood and scale). Scratch bindings: 10, the finer level or the grid read, and 11, the coarser level or the grid written, each a `RWStructuredBuffer<float>`; `main` binds input, output, uniforms and binding 11.

**Local laplacian**, one scratch buffer per level 1 to `last`, each eight planes of the level's size, laid out plane after plane: `G_l`, `B₀,l` to `B₅,l`, `O_l` (at the last level `O_last` is a copy of `G_last`). Level 0 is never stored: every pass that needs it computes it from the input, as darktable's padding is clamped addressing of the image.

| Pass | Kind | Size | Reads | Writes |
| --- | --- | --- | --- | --- |
| `ll_reduce_input` | whole-image | level 1 | the input's 5×5 footprint, each tap's L and its six curves | level 1's `G` and `B` planes (and `O` when level 1 is the last) |
| `ll_reduce`, levels 2 to `last` | whole-image | level `l` | level `l − 1`'s seven planes | level `l`'s seven (and `O` at the last) |
| `ll_assemble`, levels `last − 1` to 1 | whole-image | level `l` | level `l`'s `G` and `B`, level `l + 1`'s `B` and `O` | level `l`'s `O` |
| `main` | per pixel | the image | the pixel, level 1's `B` and `O` | the pixel |

`main` is darktable's level 0 of the assembly and its write-back in one: `v = L/100`, `lo`, `hi` and `a` from it, and `L' = 100 · (expand(O₁) + (1 − a)(curve(v, γ_lo) − expand(B_lo,1)) + a(curve(v, γ_hi) − expand(B_hi,1)))`. 2·`last` passes in all: 20 for mire1, 18 at 2400×1600. Scratch is about `32/3` bytes per padded pixel at level 0: 295 MB for mire1, 1.5 GB at 61 MP (13600×10432 padded), against darktable's `(2 + 6)` floats per pixel of every level, level 0 included, four times as much.

**Bilateral grid**, two scratch buffers of `size_x·size_y·size_z` floats, the grid ping-ponged between them:

| Pass | Kind | Size | Reads | Writes |
| --- | --- | --- | --- | --- |
| `bl_splat` | whole-image | `size_x` × `size_y` columns | the pixels within a cell of the column, each one's L | the column's `size_z` nodes, in grid A |
| `bl_blur`, along x, then y | whole-image | `size_x·size_z` × `size_y` | grid A, then B | grid B, then A |
| `bl_blur`, along lightness | whole-image | the same | grid A | grid B |
| `main` | per pixel | the image | the pixel, grid B | the pixel |

The blur's axis is its uniform's. Only `main`'s uniforms hold the detail, so a change of it leaves the grid's passes' bytes and their key unchanged (a unit test), and in the viewport a detail drag in this mode runs `main` alone (decision 15; `docs/engine.md`, Caching).

The splat is a gather, since WGSL has no float atomics: each invocation owns a column of nodes, clears it, and adds every pixel of the cells around it (those whose cell index along x is the column's or the one before, likewise y) to its two nodes along z, in storage, where only it writes; each pixel's contribution is darktable's to the node. Its sum runs in a different order from darktable's threads, a difference in the last bits.

**lavapipe's 128 MiB binding.** The local laplacian filter binds level 1's buffer whole, 32 bytes a pixel of a quarter of the padded image, and the padding doubles as the shorter side reaches each power of two. So refusals start at a shorter side of 2048 pixels, where the padding grows to 1024: 3072×2048 needs a 160 MiB level 1, and 2730×2048, 5.6 MP, is refused too, while 3:2 images below 2048 a side fit. The viewport is affected as well: a 3840×2160 fitted image needs about 198 MB. The bilateral grid at coarseness 3 and contrast 1 needs about 545 MB a grid buffer at 24 MP (2001×1334×51 nodes). Each is refused on lavapipe, naming the pass.

That is an accepted limit of software GPUs (decision 7) until image-shaped scratch binds in windows (decision 15), planned engine work before Phase 3's exit. It lifts the same limit for bloom, soften, the colour equalizer, filmic rgb and haze removal.

## Neighbourhood and scale

- **Neighbourhood radius:** the local laplacian filter's is the whole image: its coarsest level, a few pixels across, covers it. The bilateral grid's is about 2σs' for the splat and blur together, ⌈4σs⌉ by darktable's tiling overlap.
- **Scale dependence:** the local laplacian filter has no length among its parameters, but its depth follows the image's size: one level per halving of its shorter side, so the coarsest level is the same fraction of the image at every scale, and a smaller pipe loses only the finest detail levels. The bilateral grid's spatial sigma is darktable's `sigma_s / max(iscale / roi_in→scale, 1)`, `sigma_s` times the pipe's scale in Apertrust, unrounded, and the grid's sizes follow from it as darktable derives them; `sigma_r` does not scale.
- **What Apertrust computes at each scale** (decision 15; principle 2): both modes' whole-image passes run over the whole image at the pipe's scale in every pipe, never a visible region. At scale 1, an export and the 100% view, they are darktable's export exactly. The viewport is the same processing at its scale: the pyramid built on the fitted image, as many levels as its shorter side has halvings (10 at 2400×1600, against mire1's 11 at full resolution), the bilateral grid at `sigma_s · scale`. Results has how far the viewport is from the export downscaled.
- **darktable's preview and full pipes:** darktable runs the module on each pipe's region: its preview pipe on the whole image at preview size, its darkroom full pipe on the visible region at the zoom's scale, the local laplacian pyramid padded at the region's edges and the bilateral grid aligned to the region's corner (bilat passes no preview boundary to `local_laplacian`, `b = 0`). So darktable's darkroom view of local contrast depends on zoom and panning and differs from its export; Apertrust's views do not.
- **darktable's tiling:** the local laplacian filter cannot tile (`commit_params` sets `process_tiling_ready = FALSE`), so darktable's export always runs it on the whole image. The bilateral grid allows tiling (`tiling_callback`: overlap `⌈4σs⌉`, about two grid buffers of memory), each tile with a grid of its own aligned to the tile, which darktable does only when its memory runs short; the golden renders do not tile.
- **Tiling in Apertrust:** local contrast does not opt in to a first round of tiles (`Operation::tiles_whole_image_passes`, decision 15, which the tone equalizer's EIGF takes): every level of its pyramid and its whole grid need the whole image, so an export runs it whole with every operation before it, and tiles only what follows, in one round. `a_tiled_export_with_local_contrast_equals_the_whole_one` (`crates/engine/tests/tiles.rs`) holds a tiled export bit-identical to the whole one in both modes. No tile overlap is needed: `main` reads only its pixel and scratch.
- **Region-of-interest changes:** none (`modify_roi_in` and `_out` are darktable's defaults).

## Precision and special values

- **fp64 in darktable:** `ll_expand_gaussian`'s constants `4./256.`, `24.0` and `4.0` are doubles, so part of each expansion is computed in double and rounded to float on return; the kernel computes it in f32, a difference in the last bit.
- **`dt_fast_expf`** is reproduced bit for bit (an `fma` of x into the bit pattern, as darktable's arm64 build contracts it, its integer truncation, and 0 for a non-positive pattern), with its own derivative: within a binade its value is linear in x, so the derivative is that slope, `2^e · 11401300 / 2²³` for an output in `[2^e, 2^(e+1))`, which finite differences see; its 128-ulp steps are below their resolution.
- **Lightness above 100 and below 0:** darktable feeds the pyramid `L/100` unclamped, so values above 1 interpolate between the top two grey levels with `a` clamped at 1, and the bilateral grid clamps `z` to the grid. Negative lightness, which only negative input reaches, goes the same way; the bilateral grid's slice clamps its output at 0, the local laplacian filter's does not.
- **`sigma_r` 0 in the bilateral grid:** darktable computes `round(100 / 0)` and converts the infinity to an integer, which is undefined in C (`fcvtzs` saturates it on arm64, giving `z` 50, 51 nodes; x86's conversion would give 4). `prepare` takes the limit as `sigma_r` falls to 0, `z` 50, the arm64 build's value, which Rust's saturating conversion gives.
- **Images too small for the filter:** darktable's local laplacian returns without writing its output below 2 pixels a side and indexes `padded[−1]` below 4 (one level); `prepare` returns a pass of `main` that copies the pixel through Lab and back below 4 pixels a side.
- **NaN and infinity:** darktable checks for neither. The kernel relies on neither: the curve's divisions are by `σ ≥ 0.001`, the fast exponential is bounded by construction, the grid's reciprocals are of sizes at least 0.5; Lab's cube root takes negative input through its linear segment.

## CPU and OpenCL differences in darktable

- **The local laplacian filter's padded size** is rounded up to the device's work-group multiples on OpenCL (`ROUNDUPDWD`, `ROUNDUPDHT`), so its pyramid's levels have other sizes and its border replicates further right and down: darktable's GPU result is not its CPU result, and depends on the device. Its gamma levels are written `(hi + .5f)/num_gamma` inline, and `a` as `clipf(v·6 − (lo + 0.5))`, the same values.
- **The bilateral grid on OpenCL** splats with atomic adds (and is refused on devices that avoid atomics, `dt_opencl_avoid_atomics`), so its sum's order differs from run to run.
- The port follows the CPU path; the references are rendered on the CPU.

## Deliberate differences

- **The splat as a gather** (GPU plan), in its own summation order; the blur and slice in darktable's arithmetic. A difference in the last bits of each node.
- **Level 0 recomputed** from the input in every pass that reads it, rather than stored: the same values. **The 5×5 sums run in another order** at every reduce level, level 1 from the input and every coarser one: each column's vertical sum, then one horizontal sum, where darktable alternates two horizontal stencils. A difference in the last bits.
- **`ll_expand_gaussian` in f32** (Precision): the last bit.
- **`dt_fast_expf`'s multiply and add fused** (`fma`), as the reference build, darktable-cli for macOS on arm64, contracts them. An x86 build without FMA rounds the product first, so its bit pattern can differ by a step of 64, about 8e-6 relative, below what the golden cases see.
- **`sigma_r` 0 in the bilateral grid** takes the arm64 build's `z` of 50 (Precision), where darktable's C is undefined.
- **Images under 4 pixels a side** pass through the local laplacian mode (Precision), where darktable writes nothing or reads out of bounds.
- **The whole image at every pipe's scale** (Neighbourhood and scale), where darktable's darkroom processes the visible region: principle 2's same processing at each scale, not a departure from darktable's export.

## Choices

1. **One scratch buffer per level, eight planes each**, so every pass binds at most two scratch buffers, within WebGPU's eight storage buffers a stage, and the largest binding is level 1's, 221 MB for mire1 and 1.1 GB at 61 MP. Not one buffer for the whole pyramid, which binds 1.5 GB at 61 MP, nor a buffer per plane and level, which `ll_assemble`'s fourteen planes would bind past eight.
2. **Level 0 never stored** (GPU plan): darktable's padding is clamped addressing of the image, so `ll_reduce_input` and `main` read the input instead, saving the padded level 0's seven planes, 4.0 GB at 61 MP. `ll_reduce_input` converts each of its 25 taps to Lab and evaluates the six curves there, about 37 conversions per image pixel. If the pass proves slow on other GPUs, the alternative is a pass writing the input's lightness to image-shaped scratch first, one pass and 4 bytes a pixel more.
3. **Both modes end in one `main`**, their uniforms' mode choosing the path, a branch every pixel takes the same way, so one pair of derivative entry points binds as `main` does in either mode. An entry point per mode would not do: the derivative dispatch of decision 15 cannot tell them apart.
4. **Choice names from darktable's descriptions:** `bilateral_grid` ("bilateral grid") and `local_laplacian` ("local laplacian filter"), as colour balance rgb's are.
5. **The bilateral grid's slice gain kept in the kernel** (`norm` from `detail` per pixel), so `detail` is differentiable through it.

## Tool binding notes

In the first pass (decision 2) the Local contrast module is darktable's and owns every parameter (`docs/modules/README.md`): Mode, Detail (shown as darktable shows it, 100 % more than the parameter: `detail` 0 is 100 %, darktable's default 0.25 is 125 % and 1.0 is 200 %), and by mode Coarseness and Contrast (`sigma_s`, `sigma_r`) or Highlights, Shadows and Midtone range (`sigma_r`, `sigma_s`, `midtone`), one row per mode for the two shared parameters, under darktable's labels. darktable's presets: "clarity" (local laplacian, detail 0.33, highlights and shadows 0, mid-tone range 0.5) and "HDR local tone-mapping" (detail 1, highlights and shadows 0, mid-tone range 0.25).

- **What is an identity:** the local laplacian filter at highlights and shadows 1 (100 %) and detail 0 (100 % shown), within rounding; the bilateral grid at detail 0, on every pixel of non-negative lightness. darktable's defaults (detail 125 %, highlights and shadows 50 %) change the image. The Local contrast module is switched (decision 32; `docs/modules/README.md`, rule 4): off, and not planned, until a control changes, then on with those defaults, until its reset; no preset "darktable's default".
- **Changing the mode in darktable's interface** copies the other mode's sliders into `sigma_r` and `sigma_s` (`gui_changed`): the values the hidden sliders hold, 20 and 50 for the bilateral grid and 50 % for the local laplacian's highlights and shadows while the instance has been in the other mode (`gui_update`). The module's binding reproduces that, since the two parameters mean different things in each mode.
- **Derivatives:** `detail` is the first gradient slot; then `sigma_s`, `sigma_r` and `midtone` (Gradients).

## Gradients

The derivative entry points are those of `main`, the pass that writes the output, over the scratch the forward render left (decision 15), against `detail`, `sigma_s`, `sigma_r` and `midtone`, in that order:

- **Bilateral grid:** the grid does not depend on `detail`, so `main`'s derivative with respect to it is the operation's. `sigma_s` and `sigma_r` set the grid's integer geometry, piecewise constant, and their derivatives are 0. The case's slot is `detail`.
- **Local laplacian filter:** every parameter feeds the remapped pyramids, so `main`'s derivative, with the coarser levels held, is only level 0's share: the finest Laplacian coefficient's dependence on the curve. Central differences of whole renders see every level. The gap is real and understood, and the case names no slot (`ports.rs`), so the routine check stays meaningful.

The local laplacian mode declares no gradient slot until pyramids of the curve's four parts are built, when local contrast's controls are to be fitted (decision 11). Since the filter's output is affine in `detail`, `sigma_s` and `sigma_r` for a fixed mid-tone range (the curve is, and the pyramid and its assembly are linear in the remapped images), those pyramids would give `main` the exact derivatives and make Detail, Shadows and Highlights drags one per-pixel pass, at four times the remapped pyramids' memory. Input derivatives through the mode hold its pyramids constant, their error measured with the matching slice.

Results (Apple M4, step 1e-3, the sum of output RGB as the loss):

| Case | Parameter | Case: relative error | Case: forward vs reverse | Synthetic: relative error | Synthetic: forward vs reverse |
| --- | --- | --- | --- | --- | --- |
| `bilat-bilateral` | `detail` | 1.8e-5 | 5.5e-8 | 1.7e-5 | 5.6e-8 |

The local laplacian filter's gap, measured with its four slots named for the purpose (not in `ports.rs`): against finite differences of whole renders, relative errors of 0.89 (`detail`), 1.1 (`sigma_s`), 1.0 (`sigma_r`) and 0.99 (`midtone`) on the case, and 0.25, 0.63, 1.3 and 0.10 on the synthetic sweep, while forward and reverse mode agree per pixel within 3.6e-6 and 3.3e-7. The derivatives are right for what they are: `crates/golden/tests/bilat.rs` checks them against central differences of renders whose whole-image passes keep the settings and whose `main` alone moves, within 1.0e-5, 7.4e-6, 3.7e-4 and 1.9e-4 on a 64×48 image, and forward against reverse mode within 1e-4.

Known non-smooth spots: the curve's joints at `c = ±2σ` and `0` (C¹, so only second derivatives jump), the fast exponential's binades, the interpolation weight's clamp at the top and bottom grey levels, and the bilateral slice's clamp at 0.

## Results

**Speed** (Apple M4, a render of the one-instance plan, every pass, GPU timestamps, 3908×2602): the local laplacian filter takes 58 to 63 ms a render (the medians of three runs; minimums 41 to 47 ms), of which, warm, `ll_reduce_input` takes 25 ms, the nine `ll_reduce` 6.7 ms, the nine `ll_assemble` 6.5 ms and `main` 5 ms. The first pass's 25 taps, each converted to Lab with its six curves, are the cost to cut first (Choices, 2). The bilateral grid takes 12 to 14 ms (8 to 11): the splat 7 to 8 ms, the three blurs under 1 ms on its 79×53×6 grid, `main` 3 to 8 ms.

**Identities** (`crates/golden/tests/bilat.rs`, a 64×48 image of greys and colours from 0.002 to about 11): the local laplacian filter at highlights and shadows 1 and detail 0 returns its input within 2.1e-6 of each pixel's largest channel, the bilateral grid at detail 0 within 1.7e-6: the Lab round trip's rounding.

**The viewport against the export** (decision 15's target, `the_viewport_is_within_decision_80s_target_of_the_export`, ignored, run by hand): on each case's input, the operation on the fitted image against the export downscaled by the viewport's box filter, mean ΔE2000. The table takes the identity's baseline, 0, since local contrast has no per-pixel part to leave the filtering out of.

| Case | 2400×1598 | 1/2, 1954×1301 | 1/4, 976×650 | 1/8, 488×325 |
| --- | --- | --- | --- | --- |
| `bilat` (local laplacian, darktable's defaults) | 0.131 | 0.056 | 0.108 | 0.177 |
| `bilat-bilateral` (detail 200 %, 1.0) | 0.093 | 0.132 | 0.272 | 0.478 |
| the same at darktable's default detail, 125 % (0.25), by hand | 0.023 | 0.032 | 0.065 | 0.115 |
| Target | 0.15 | 0.15 | 0.4 | 0.4 |

The local laplacian filter meets the target at every scale, and so does the bilateral grid at darktable's default detail. At detail 200 % the bilateral grid exceeds it at 1/8: its slice adds a nonlinear function of each pixel's own lightness, so the export's mean over a footprint of its pixels' slices differs from the viewport's slice at their mean lightness wherever a footprint averages an edge or a texture, in proportion to the detail.

The target applies as decision 15 measures it: the baseline is the per-pixel gap left once the whole-image result is exact, measured with the whole-image passes at full resolution and `main` at the viewport's scale through a setting only the tests reach, and the target applies to the rest. That baseline is still to be measured; until it is, the test bounds the 1/8 view at detail 200 % by 0.5, a regression check.

## Test plan

- **Golden cases:** `bilat`, darktable's test 0002-local-contrast on mire1.cr2, the local laplacian filter at its defaults after exposure +1 EV, unchanged; `bilat-bilateral`, test 0002 edited to the bilateral grid with the coarseness and contrast darktable's sliders give the mode (50 and 20) and detail 200 % (1.0), so the module's effect is well above the tolerance's mean; no darktable test covers the mode. Both `Tolerance::DARKTABLE`, both one-instance plans of the engine (decision 15).
- **Unit tests** (`crates/op/bilat`): the schema's defaults are darktable's, and its values round-trip; every parameter reaches its setting; the pyramid's geometry (mire1's 11 levels, `max_supp` 1024, level sizes, one level per halving, none below 4 pixels); the bilateral grid's geometry equals darktable's on test 0002's edited case (79×53×6, σs' 50.102566), its clamps and `sigma_r` 0's limit, and its spatial sigma follows the pipe's scale; `prepare`'s passes, sizes, bindings and scratch per mode, every pass within eight storage bindings, only `main` per pixel and writing the output, binding the same resources in either mode; the bilateral grid's detail reaches only `main`'s uniforms; small images pass through; the uniforms' layout; the manifest says the operation is differentiable, and the operations' own test that its kernel agrees.
- **In `ports.rs`:** test 0002 reads as version 3 with the local laplacian filter at darktable's defaults and no gradient slot, the edited case as the bilateral grid with `detail` its slot; versions 1 and 2 migrate as `legacy_params` does, and other sizes and versions are refused.
- **On the GPU** (`crates/golden/tests/bilat.rs`, run in CI): both modes' identities (Results); darktable's defaults change the image and keep alpha; output stays finite for black, 1e-30, negative, very bright (10⁶) and saturated input at the settings' ends in both modes (detail −1 and 4, highlights and shadows 0 to 100, mid-tone range 0.001 and 1, coarseness and contrast 0 to 100); images of 1×1 to 5×7 run, those under 4 pixels a side passing through; the local laplacian filter's derivatives against `main` alone (Gradients). And, ignored, run by hand: the viewport's target (Results). In `crates/engine/tests/tiles.rs`, run in CI: a tiled export through the default pipeline and local contrast, darktable's defaults and the bilateral grid at coarseness 50, contrast 20 and detail 200 %, equals the whole one bit for bit at every tile size, in one round.
