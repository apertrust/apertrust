# Port spec: grain

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/grain.c` `commit_params()` and `process()`, with the working space's conversions around a Lab module from `src/common/iop_profile.c` (`_transform_rgb_to_lab_matrix`, `_transform_lab_to_rgb_matrix`) and `src/common/colorspaces_inline_conversions.h` (`dt_XYZ_to_Lab`, `dt_Lab_to_XYZ`). darktable has no OpenCL grain: `grain.c` has no `process_cl` and `data/kernels/` no grain kernel. The reference is darktable's CPU path.

Status: ported.

- **Coverage:** every continuous parameter (coarseness, strength, mid-tones bias) and both of darktable's noise paths, the single position at full resolution and the 21-position lattice it averages away from it, in Slang (`crates/op/grain`), with its manifest and schema.
- **The offset** darktable derives from the file name is no parameter: the golden cases set darktable's through `Settings`, and the operation renders every image at a provisional offset of 0. Planned: a seed stored in the edit (Deliberate differences).
- **Planning:** in `OPERATIONS` with `default_enabled` off: an unedited photo does not run it, and the session plans it only while a module's binding sets one of its parameters (decision 28). The Grain module does, while its strength is off 0 (`docs/modules/grain.md`).
- **Checks:** darktable's grain is deterministic, and the port reproduces it: five golden cases pass with a mean ΔE2000 below 0.001.

Golden result (five cases, four on test 0059's mire1.cr2, 3908×2602, one on hlrecovery.arw, 5492×3672, Apple M4, 48-byte pass header): every case passes, ΔE2000 mean 0.0008 at most, max 0.095 at most (Results). Gradients: the golden check passes the strength on four cases and on the synthetic sweep, at worst 3.8e-3, forward and reverse mode within 2.3e-6; the coarseness and the mid-tones bias, which that check cannot resolve, follow a float64 model per pixel in a GPU test, on at least 99.6% of pixels within 1%, through the lattice too (Gradients).

## Purpose

Film grain for the photographer: a fine, irregular texture over the image, its size set like a film's speed in ISO, stronger in the mid-tones than in the shadows and highlights, as silver grain prints on photographic paper.

Algorithmically, three octaves of Gustavson's 3D simplex noise at the pixel's position, normalised by the image's shorter side, are scaled by the strength and passed through a photographic paper's response, a logistic density curve that the mid-tones bias makes steeper, to a change of Lab lightness. It is the Effect tab's Grain module (background only, `docs/modules/README.md`), display-referred, after the tone mapper.

## Pipeline

- **darktable position:** 65.0 in the v3.0 and v5.0 orders, after the tone mappers (sigmoid 45.3, AgX 45.5, filmic rgb 46.0) and monochrome (64.0), before soften (66.0), split-toning (67.0), vignette (68.0) and output colour (70.0). The manifest's slot is the display stage at 65.0, its domain display, darktable's place: after the tone mapper, and so after RGB curve and RGB levels (working 42 and 43, decision 22), and before vignette, so the curve never reshapes the grain and the vignette darkens it, as in darktable (decision 22; `docs/engine.md`, Order).
- **Domain:** display-referred. darktable's description: "non-linear, Lab, display-referred" in, out and in processing; "creative" purpose. Test 0059 has no tone mapper, so there it runs on scene-referred values; the `grain-agx` case runs it after AgX, where Apertrust will.
- **Input and output:** darktable's grain is a Lab module (`default_colorspace` returns `IOP_CS_LAB`): its pipe converts the working space to Lab before it, with the working profile's matrix to XYZ D50 and `dt_XYZ_to_Lab`, and back after it. Apertrust's pipeline stays in working RGB, linear Rec2020, so the kernel makes both conversions itself, with the same matrices (`WorkingProfile::linear_rec2020`) and darktable's Lab formulas, in the shared colour library (`xyz_to_lab_d50` and `lab_to_xyz_d50` in `color.slang`, the CPU twins in `crates/color`). Only L changes; a and b stay, and alpha passes through.
- **Assumes earlier modules:** nothing about colour. Its noise assumes the size of its input at full resolution, darktable's `piece->buf_in`: the image after any crop or geometry module before it, as for vignette.
- **Layer support:** background only, as the Grain module is, though the operation is per pixel and could run on a layer.

## Parameters

darktable params version 2, 16 bytes: `channel` (`_dt_iop_grain_channel_t`, i32), `scale`, `strength`, `midtones_bias` (f32).

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| `scale` | `scale` | float | 20/213.2 to 6400/213.2 (0.0938 to 30.02) | 1600/213.2 (7.505) | "coarseness": the grain's size; darktable's slider shows it as ISO, `scale · GRAIN_SCALE_FACTOR` (213.2), from 20 to 6400 | Grain |
| `strength` | `strength` | float | 0 to 100 | 25 | The grain's strength, in percent | Grain |
| `midtones_bias` | `midtones_bias` | float | 0 to 100 | 100 | "mid-tones bias": how much the paper response makes the grain fall off in the shadows and highlights; 0 a nearly linear response, 100 a strong fall-off | Grain |
| none | `channel` | hue, saturation, lightness, RGB | | lightness | No effect: darktable's `process()` applies every channel to lightness, and its GUI has no channel widget | not ported (Deliberate differences) |

The defaults are darktable's introspection defaults (the module has no `init`, `reload_defaults` or presets). darktable's `process()` also starts the noise at an offset it computes from the file name (Determinism). That is no field of darktable's, so it is not a parameter (decision 21) and an edit saves none: the operation renders every image at `PROVISIONAL_OFFSET`, 0, and `Settings::offset` lets the golden cases set darktable's (Deliberate differences).

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 | `channel`, `scale`, `strength`, 12 bytes | `midtones_bias = 0`, "it produces the same results as the old version" |
| 2 | adds `midtones_bias` | current |

`ports.rs` reads both: test 0059 is version 2, and the `grain-v1` case is version 1, which darktable migrates; that case passes, so the migration is darktable's.

## Determinism

darktable's grain is deterministic: no random state, no thread dependence. Each pixel's noise is a function of its position, the offset and the parameters, through a fixed table (Ken Perlin's permutation, `permutation` in `grain.c`, doubled into `perm` by `init_global`) and fixed gradients (`grad3`, the twelve edges of a cube). A float64 emulation of `process()` reproduces darktable's render of test 0059 within 1.2e-6 per channel everywhere. Two things make it depend on more than the image and its settings:

- **The offset depends on the file name and on the width darktable renders.** `hash = _hash_string(filename) % (int)fmax(roi_out->width * 0.3, 1.0)`: a DJBX33X hash of the file name's bytes from its last to its first ("to get rid of static grain when creating a video from a sequence of images"), modulo 0.3 times the region's width. An export at full resolution renders the whole image, so its offset is fixed by the name and the image's width (758 for mire1.cr2 at 3908 wide, 150 for hlrecovery.arw at 5492). darktable's previews render other widths, the darkroom's main view the visible region's, so darktable's grain in the darkroom differs from its export and changes with the zoom. Apertrust's offset is the same for the whole image at every scale: provisionally 0 for every image, darktable's in the golden cases (Deliberate differences).
- **The hash reads `char`, which is signed on x86-64 and on Apple's arm64 and unsigned on Linux on arm64**, so a name with a byte above 127 hashes differently there. `hash_string` follows the signed `char`.

## Setup on the CPU (`commit_params`, the start of `process`)

`commit_params` copies the parameters and fills the paper response's table, `evaluate_grain_lut(mb)`: 128 × 128 f32 entries, row `j` the lightness `l = j / 127`, column `i` the grain `gu = i / 127 − 0.5`, the entry `100 · (P(gu + P⁻¹(l)) − l)`: the change of lightness, in L, when the grain moves the paper's exposure by `gu` (Per pixel). `process()` then derives, before its loop, with `W` and `H` the input's size at full resolution (`piece->buf_in`) and `s` the pipe's scale (`roi_out->scale`):

1. **The offset**, `hash` above.
2. **The strength** as a fraction, `strength / 100`.
3. **The shorter side**, `wd = min(W, H)`.
4. **The zoom**, `(1.0 + 8 * scale / 100) / 800.0`: from 0.00126 at 20 ISO to 0.00425 at 6400. darktable's release builds compile with `-ffast-math` (`src/CMakeLists.txt`), which makes `8 * scale / 100` the f32 product `scale * 0.08f`; the rest is double.

   The two differ by an f32 ulp at test 0059's coarseness, and the offset magnifies it: with 758 shorter sides, the noise's coordinate is about 3e5, and that ulp moves darktable's grain by 0.02 of a noise cell, a residual of 0.048 in the noise (standard deviation), about 0.7 L. Two things show darktable uses the product:
   - The `grain` case is the only one whose coarseness tells the two forms apart (at 100, 1150, 1600, 2300 and 3200 ISO, the other cases' and the default, both give the same f32), and it matches darktable's render with `scale * 0.08f` (the emulation within 1.2e-6, the kernel at ΔE2000 mean 0.0006), not with the correctly rounded quotient or in double.
   - darktable's shipped `libgrain.so` disassembles to a multiply by 0.08f, and `strength / 100.0f` to `strength * 0.01f` (which nothing magnifies, so the kernel divides).

   `the_zoom_is_darktables_fast_math_product` only checks that the two forms differ at test 0059's coarseness.
5. **The lattice:** on when the pipe is not fast (`dt_pipe_is_fast`, darktable's thumbnails) and `|s − 1| > 0.01`; its width `filtermul = piece->iscale / (s · wd)`.

Apertrust's `prepare` (`crates/op/grain/src/setup.rs`) checks the settings, refusing values that are not finite or lie outside darktable's ranges, and passes darktable's parameters to the kernel, with:

- **the offset's term per octave**, `offset · f_k / zoom` with darktable's zoom, in f64, reduced by the noise's period of 768 (Precision), three f32 values below 768, from `Settings::offset`: the provisional 0 from the operation's parameters, darktable's offset in the golden cases;
- **the working space's matrices**, RGB to XYZ D50 and back;
- **darktable's `perm`** as the kernel's table at binding 3, 512 `u32`.

The zoom, the strength's fraction and `wd` are the kernel's: `wd`, `s` and the lattice's switch come from the pass header (`full_width`, `full_height`, `scale`), and the paper response's entries are computed where they are read (Deliberate differences).

## Per pixel (`process`)

darktable (`grain.c:process`), for the pixel at column `i` and row `j` of the region, in Lab:

1. **Position:** `x = (roi.x + i) / s / wd`, `y = (roi.y + j) / s / wd` (double): normalised by the shorter side at full resolution, so the grain is the same at every scale.
2. **Noise** (`_simplex_2d_noise`): `Σ_k a_k · noise3(x' · f_k / zoom, y' · f_k / zoom, k)` over three octaves, `f = (0.4910, 0.9441, 1.7280)`, `a = (0.2340, 0.7850, 1.2150)` ("to match the power spectrum of real grain scans"), with `x' = x + hash`, `y' = y`, stored as f32. With the lattice, the mean of the same at 21 positions `(x + dx_l, y + dy_l)`, `dx_l = (l / 21) · filtermul`, `dy_l = frac(l · 34 / 21) · filtermul`, a rank-1 lattice of Fibonacci numbers, accumulated in f32.
3. **`noise3`** (`_simplex_noise`): Gustavson's 3D simplex noise in double. Skew by `(x + y + z) / 3` to the cell `(i, j, k) = FASTFLOOR(p + skew)`, unskew by `(i + j + k) / 6`, order the offset's components to choose the simplex's second and third corners, hash each corner's gradient as `perm[i + perm[j + perm[k]]] % 12`, and sum `(0.6 − |d|²)⁴ (g · d)` over the corners where `0.6 − |d|² ≥ 0`, times 32.
4. **The grain:** `u = (noise · strength / 100) · GRAIN_LIGHTNESS_STRENGTH_SCALE` (0.15), f32.
5. **The lightness change** (`dt_lut_lookup_2d_1c`): bilinear in the table at `x = (u + 0.5) · 127` and `y = L / 100 · 127`, each clamped to [0, 127], the cell's corner at most 126.
6. **Output:** `L' = L + ΔL`; a and b copied. darktable does not write the output's fourth channel.

The paper response, with `δ = GRAIN_LUT_DELTA_MAX · exp(mb / 100 · ln GRAIN_LUT_DELTA_MIN) = 2 · 10^(−4 mb / 100)`, from 2 at no bias to 2e-4 at full bias, and a paper gamma of 1: `P(e) = (1 + 2δ) / (1 + exp(4 (0.5 − e) / (1 + 2δ))) − δ` and `P⁻¹(d) = −ln((1 + 2δ) / (d + δ) − 1) · (1 + 2δ) / 4 + 0.5`. A large δ makes `P` nearly linear over [0, 1], so the grain moves every tone alike; a small one makes it a steep logistic, so the grain's change of lightness falls off towards black and white.

The kernel (`slang/grain.slang`) does the same on RGB, converting to Lab and back itself, with the pixel's position in the whole image, `X = origin_x + id.x`, `Y = origin_y + id.y`: `x = X / s / wd`. Per octave the position is `x · f_k / zoom + o_k` with `o_k` the CPU's offset term, both coordinates less a whole number of periods. Every decision and table read of `noise3` (the cell, the simplex's corners, their gradients) happens first, in `locate()`; the differentiable `simplex()` gets them as arguments. The table's four entries around `(x, y)` are computed from the formula above, in darktable's f32 expressions, rather than read, and the cell is found before the differentiable part. Alpha passes through.

## Neighbourhood and scale

- **Neighbourhood radius:** none: per pixel. The manifest's radius is 0; a tile needs no border.
- **Position and scale:** the noise depends on the pixel's position relative to the input at full resolution, normalised by its shorter side, and divided by the pipe's scale, so the grain covers the same part of the image at every scale. Away from full resolution, darktable averages the noise over the pixel's footprint, the lattice above, of width `piece->iscale / (s · wd)`. The kernel takes `wd` and `s` from the pass header and adds the header's origin to its invocation, so a tile, a region and the viewport compute what the whole image would at their scale (decision 13; `crates/golden/tests/grain.rs` checks a tiled export against the whole one bit for bit). The lattice is checked against the GPU test's transcription of darktable's `process()`, itself checked against `grain.c`, at scales 0.5 and 0.3, through the viewport and through the kernel at a header's scale. No darktable render covers the lattice, since the golden cases render at full resolution. Planned: a case rendered by darktable-cli at a reduced `--width` with `--hq false`, which runs darktable's pipe at that scale, its lattice and its hash of the reduced width included, with the harness running the kernel at the same scale.
- **`piece->iscale`:** 1 in darktable's full pipe, the darkroom's main view, which the kernel follows; darktable's preview pipe, which renders from a smaller mipmap, has its mipmap's iscale, which widens its lattice by that factor. Apertrust's viewport has no mipmap: its scale is relative to the image at full resolution, as darktable's full pipe's.
- **Region-of-interest changes:** none. `grain.c` has no `modify_roi_in`, `modify_roi_out` or `tiling_callback`, and its flags (`IOP_FLAGS_INCLUDE_IN_STYLES | IOP_FLAGS_SUPPORTS_BLENDING`) do not allow tiling, so darktable never tiles it.
- **Full resolution in the header:** the engine fills `full_width` and `full_height` with the developed image's size, darktable's `buf_in` while no crop or geometry operation runs before grain; the same caveat as vignette's (`docs/ports/vignette.md`, Neighbourhood and scale).

## Precision and special values

- **fp64:** darktable's noise is double throughout; the kernel's is f32. Two things keep it close:
  - **The period.** In darktable's skewed lattice, a step of 768 along x moves the cell by (1024, 256, 256), whole turns of the 256-entry permutation, and leaves the point's place in its cell unchanged; likewise along y. So `noise3(x + 768, y, z) = noise3(x, y, z)` exactly, and the kernel reduces both coordinates by 768 (`wrap`), keeping them below 768 where f32 resolves 6e-5 of a cell.
  - **The offset's term in f64.** darktable adds the offset to x in double: 758 shorter sides is about 3e5 cells, where f32 resolves 0.03 of a cell. `prepare` computes `offset · f_k / zoom` and reduces it by 768 in f64, then passes it as f32.
  - What is left: the pixel's term `x · f_k / zoom` rounds to about 2e-4 of a cell at the finest grain, where positions reach 2000 cells. Over test 0059's image (6400 ISO, offset 758) the kernel's noise, emulated in f32, lies within 0.0064 of darktable's, mean 6.2e-5, 99.99% within 6.6e-4: at full strength at most about 0.1 L. The golden cases' largest ΔE2000 is 0.095.
- **The simplex's jumps:** Gustavson's 3D noise sums a corner's `(0.6 − |d|²)⁴` while it is positive, but the radius 0.6 reaches past a simplex's own corners, so the noise jumps slightly where a point crosses from one simplex to the next: in one octave by 6.5e-4 at the median and 4.6e-3 at most, over 95 crossings sampled. darktable's noise has the jumps, and so does the kernel's. They matter where f32 puts a point in another simplex than darktable's f64 does (part of the 0.0064 above), and in finite differences of the coarseness (Gradients).
- **`FASTFLOOR`:** one below a whole number that is not positive (`(int)x − 1` for `x ≤ 0`). Coordinates here are never negative, so it differs from `floor` only at 0, the origin of an octave at offset 0, where both cells give the noise 0. The kernel uses `floor`.
- **The paper response's entries** are computed in the kernel in darktable's f32 expressions, with the GPU's `exp` and `log`, within about 1e-5 L of darktable's table. darktable's own entries cancel badly, `P(P⁻¹(l) + gu) − l` for a small `gu`: their f32 rounding, about 1e-5 L, is the reason the mid-tones bias's finite differences cannot be resolved (Gradients).
- **Strength 0 is not a no-op:** at `u = 0` the lookup falls half way between the table's columns 63 and 64, `gu = ∓1/254`, whose mean is not 0, from the paper response's curvature: 1.3e-4 L at bias 0, 1.15e-3 L at 50 and 1.2e-3 L at 100, at most. The kernel's round trip to Lab and back adds f32 rounding, about 1e-7 of each channel (Tool binding notes).
- **Clamps:** `x` is clamped to the table at `|u| ≥ 0.5`, which the grain never reaches (`|noise| < 2.3`, so `|u| ≤ 0.34`); `y` at L below 0 or above 100, where darktable's lookup reads its first or last row.
- **NaN, infinity and negative values:** the noise and the lookup are finite for any finite position and lightness; a negative or very bright pixel goes through Lab's linear segment or its cube root and back. Output stays finite for 0, 1e-30, ±1e6 and negative input at every setting the tests try.

## CPU and OpenCL differences in darktable

None: darktable has no OpenCL grain, and runs it on the CPU even with OpenCL on. Apertrust follows the CPU path.

## Deliberate differences

- **The offset is not a parameter, and provisionally 0.** darktable starts the noise at a hash of the file name modulo 0.3 times the width it renders, so its previews and its export use different offsets. An operation has no file name (`ImageContext`), and an offset is no field of darktable's (decision 21), so the operation renders every image at `PROVISIONAL_OFFSET`, 0, the same at every scale: Apertrust's viewport shows its export's grain, and every image the same pattern. `Settings::offset` lets the golden cases set darktable's offset for an export at full resolution (`darktable_offset`).
- **Planned: a seed stored in the edit,** its default darktable's file-name hash, `darktable_offset(name, width)`, through the image context, which gains the file name. An unedited photo then renders as darktable exports it, and the frames of a sequence differ, as darktable intends; an edited photo keeps its grain when the file is renamed. The stored value, which darktable does not have, is an agreed difference (decision 63). An imported darktable edit gets darktable's hash.
- **The coarseness's derivative holds the offset** (Gradients): the grain scales about the image's top-left corner, while the render slides the noise field as darktable's does, an understood gap.
- **Positions reduced by the noise's period, and the offset's term computed in f64 on the CPU:** the same noise as darktable's within f32 rounding (Precision).
- **The paper response's entries computed where they are read**, rather than a 128 × 128 table: the same values within f32 rounding, and their derivatives reach the mid-tones bias.
- **No `channel`:** darktable's `process()` ignores it, and its GUI has no widget for it; `ports.rs` reads it and the `grain-channel` and `grain-v1` cases, hue and RGB, pass against darktable.
- **The lattice's iscale is 1**, darktable's full pipe's; darktable's preview pipe uses its mipmap's (Neighbourhood and scale). Its fast pipes, which skip the lattice, have no counterpart.
- **Values outside darktable's ranges are `NotPorted`**: at a mid-tones bias near 1000, δ leaves f32's range and darktable's table holds infinities.
- **Alpha passes through:** darktable's `process()` leaves the output's fourth channel unwritten.

## Tool binding notes

In the first pass (decision 2) the Grain module is darktable's: coarseness, strength and mid-tones bias. As built, strength starts at 0 so an untouched module leaves the plan, darktable's 25 % its "darktable's default" preset (`docs/modules/README.md`). Under decision 32 the module is to be switched (planned): off until a control changes, then on with darktable's 25 %, 1600 ISO and 100 %, until its reset.

- **The offset** has no control: provisionally 0 for every image, until the planned seed (Deliberate differences).
- **Strength 0 is not a no-op** in the operation: darktable's lookup at `u = 0` moves lightness by 1.3e-4 L at mid-tones bias 0, 1.15e-3 L at 50 and 1.2e-3 L at 100, and the kernel keeps it. As built, the operation is left out of the plan while Strength is 0 (decision 28's `edit::instances`): Grain's binding sets nothing then, whatever the others hold, which is exact and saves the pass. Once the module is switched (decision 32, planned), it is off until a control changes and then renders whatever it holds, a strength of 0 included, as darktable's does.

## Gradients

The derivative entry points differentiate against the three continuous parameters, in darktable's units, named by the uniform's gradient slots: 0 `strength`, 1 `scale`, 2 `midtones_bias`; the fourth slot is empty. `main` reads the parameters without slots.

**How the kernel differentiates the noise.** The noise has one differentiable input, the coarseness, through the zoom. Its derivatives are written out in forward mode (`noise_forward`, `noise_backward`, which call `fwd_diff(octaves)`): reverse mode through the lattice's loop of 21 positions would need Slang's checkpoint arrays, written at a run-time index (decision 10). Every table read and simplex decision is made in `locate()`, outside the derivative, so no branch decides which table entries a differentiable function reads, and the backward pass never recomputes a simplex. The paper response's cell is found from the forward values and passed in, so both derivative passes interpolate where the forward pass did.

**A Slang bug.** slangc 2026.20 fails with "internal error: unexpected IR opcode during code emit" on the forward derivative of a function that builds a vector from a `no_diff` scalar parameter (`float3(xy, z)` with `no_diff float z`; with a literal it compiles; to WGSL and to SPIR-V alike). The kernel's `simplex()` takes the octave `z` as an ordinary parameter instead; its derivative is 0, since every caller passes a constant. A minimal repro: a `[Differentiable] float f(float2 xy, no_diff float z) { float3 p = float3(xy, z); return dot(p, p); }` under `fwd_diff`. It is to be reported to Slang with this repro once Apertrust's repository is public.

**The coarseness's derivative holds the offset.** The kernel's position is `x · f_k / zoom + o_k`, with `o_k` the CPU's offset term at darktable's zoom; moving the coarseness moves the zoom, not `o_k`, so the derivative is the grain's at a fixed offset, scaling about the image's top-left corner. darktable's own function of its coarseness slides the noise by `offset · f_k / zoom` as well: with offset 758, a change of 1e-4 in the coarseness moves the finest octave by 0.7 of a cell at 6400 ISO and by 8 cells at 100 ISO, so that function turns over within such changes, and no optimiser could use its derivative. The golden check moves the kernel's `scale` and leaves the offsets (`ports.rs`), consistent with what the kernel differentiates.

The render keeps darktable's behaviour, the noise field sliding with the coarseness by darktable's offset, and the derivative's held offset is an understood gap. Holding the offset in the render too is a candidate for after the migration.

**What `apertrust-golden gradient` can check.** Its loss is the output's R + G + B, through finite differences of the f32 forward pass at a step of 1e-3:

- **The strength** passes everywhere but on `grain-fine`'s synthetic sweep. Its derivative is mostly per-pixel terms of both signs (the noise's), and what is left, from the paper response's curvature, is small. The check's errors, about 1e-3, are consistent with the f32 rounding of `strength / 100`: at strength 100 the kernel's steps are 1.00001 and 0.99999 in f32, 0.14% further apart than 2e-5, and test 0059 shows 0.12%; float64 gives at most 8.6e-7 (below). At 100 ISO the grain is white noise at the sweep's pixels, its total there only 1.0194 in float64 (the GPU's reverse mode 1.0201), and the f32 rounding of the sweep's brightest levels, up to 4, puts 2.9e-2 into the finite differences at a step of 1e-3, 1.0e-2 at 1e-2 and 2.6e-3 at 3e-2: rounding, not a kink. So `grain-fine` names no slot; on its own image the strength passes at 1.1e-4.
- **The mid-tones bias** cannot be checked by finite differences of an f32 forward pass. A step of 1e-3 moves the table's entries by about 2e-6 L, below their own f32 rounding, about 1e-5 L, and every pixel reading an entry shares its rounding. A float32 emulation of the check on test 0059's image, with darktable's expressions, misses by 23% (bias 11.5), 33% (52.7), 120% (0) and 410% (100) at the cases' settings, and anywhere from 0.3% to 1300% when they move a little: rounding, not a kink. With a cancellation-free form of the same entry, `100 (l + δ) E (1 − e^(−a gu)) / (1 + E e^(−a gu))`, `E = ((1 − l) + δ) / (l + δ)`, `a = 4 / (1 + 2δ)`, it misses by 0.6% to 6.5% at biases up to 52.7, and by 84% to 220% at 100: still too much to check, so the port keeps darktable's expressions. On test 0059, at full bias, the GPU's finite differences give +56.1 against reverse mode's −5.2335; float64 finite differences of darktable's whole formula, over the same image, give −5.23354. The GPU test checks this derivative against float64.
- **The coarseness** cannot be checked on a whole image's sum. Its derivative there is what is left of per-pixel terms of both signs, 2.4e4 times larger in total on test 0059, so the finite differences' small errors per pixel, from the noise's curvature and the table's kinks, do not cancel as the terms do; and the simplex's jumps add errors that grow as the step shrinks, since fewer pixels cross a simplex but each counts for `jump / step`. A float64 emulation of the check on test 0059, through darktable's whole formula, misses by 0.75% at a step of 1e-2, 1.4% at 1e-3 and 72% at 1e-4; with a continuous noise (radius 0.5) still by 1.4% at 1e-3. On the GPU it fails on test 0059 at 1e-3 (2.4e-2) and passes at 1e-2 (1.2e-3), and on the sweep the other way round (4.4e-3, 3.9e-2).

So every case names the strength alone, `grain-fine` nothing (`ports.rs`), and `crates/golden/tests/grain.rs` checks all three parameters per pixel: the kernel's forward mode against central differences of a float64 model of the kernel's own function (the noise with the offset held, the paper response's entries from the formula, Lab in float64), at steps of 1e-5 (strength, bias) and 1e-7 (coarseness), where a pixel rarely crosses a simplex. A pixel agrees when it lies within 1% of the model's derivative, or of the image's mean where that is small; the check needs 99% of pixels and channels to agree (a wrong derivative fails nearly all of them), and forward and reverse mode within 1e-4 per pixel, as the golden check measures it. The lattice's path is checked the same way at scale 0.5, through the kernel runner's `dispatch_derivative_with`, which takes a pass header; the golden harness's `gradient` dispatches at scale 1. Planned: a harness change adds float64 references for the parameters f32 finite differences cannot resolve, as the mid-tones bias and the coarseness here.

Results, Apple M4, step 1e-3, relative error of reverse mode against central finite differences, and the largest per-pixel difference between forward and reverse mode:

| Case | Strength, own image | Forward vs reverse | Strength, synthetic sweep | Forward vs reverse |
| --- | --- | --- | --- | --- |
| `grain` | 1.2e-3 | 2.3e-6 | 1.1e-3 | 2.3e-6 |
| `grain-fine` | not named (1.1e-4) | 1.7e-6 | not named (2.9e-2) | 1.3e-6 |
| `grain-channel` | 1.3e-3 | 2.3e-6 | 2.9e-3 | 9.7e-7 |
| `grain-v1` | 1.2e-3 | 1.9e-6 | 6.8e-4 | 9.8e-7 |
| `grain-agx` | 8.3e-4 | 1.6e-6 | 3.8e-3 | 1.7e-6 |

A float64 emulation of the strength's check, with the parameters moved by ±1 (strength) and ±0.5 (bias), gives at most 8.6e-7 on every case's image: no kink near any case's settings.

Per pixel against the float64 model (`derivatives_follow_the_float64_model_per_pixel`, `the_lattices_derivatives_follow_the_float64_model`, a 96 × 64 gradient image), the share of pixels and channels within 1%, and forward against reverse mode:

| Settings | Strength | Coarseness | Mid-tones bias | Forward vs reverse | Lattice, scale 0.5: strength, coarseness, bias |
| --- | --- | --- | --- | --- | --- |
| test 0059 (6400 ISO, 100%, 100%, offset 758) | 99.96% | 99.92% | 100.00% | ≤ 1.8e-6 | 99.99%, 100.00%, 100.00% |
| 100 ISO, 37.3%, 52.7% | 99.94% | 99.96% | 99.98% | ≤ 2.1e-6 | 100.00%, 99.63%, 100.00% |
| 1600 ISO, 61%, 23% | 99.98% | 99.93% | 99.99% | ≤ 3.5e-6 | 99.98%, 99.97%, 100.00% |

The margin is smallest for the lattice's coarseness at 100 ISO, 99.63% against the 99% the test needs. Each pixel's 63 noise evaluations can each fall in another simplex in f32 than in float64, and a GPU with less precise `exp` and `log` than the M4's, lavapipe or WARP, may put more of them there.

Known non-smooth spots, and what an optimiser would use instead:

- **The simplex's jumps:** the noise is discontinuous where a point crosses into the next simplex, so the coarseness's derivative is defined almost everywhere but its function jumps. Gustavson's radius of 0.5 would make the noise continuous, at the cost of darktable's grain.
- **The coarseness at all:** the grain's pattern is chaotic in its size; its derivative is exact for small moves, but an optimiser fitting grain would fit its statistics (strength, size as a power spectrum), not pixels.
- **The paper response's table:** bilinear, so kinks in the grain and the lightness at its cells' edges, every 1/127.
- **The lattice's switch** at a scale 0.01 from 1, and its clamps: not parameters.

## Results

Apple M4, 48-byte pass header (decision 14's), every case `Tolerance::DARKTABLE`. The kernel reproduces darktable within the f32 noise's rounding. A pass takes 3.85 ms over 10 MP at best (medians 3.85 to 4.7 ms across runs, as the passively cooled M4 throttles), exposure's time, so memory-bound, and 7.6 ms over 20 MP; with `--repeat 32`, 0.78 ms per application of the per-pixel function after the noise (Lab, the paper response, back). The lattice costs 72.7 to 76.0 ms per pass over 3908 × 2602 of output at scale 0.5 (the median of 20 timed passes, in three runs), against 3.85 ms for the single position at scale 1: 19 times as much, its 63 noise evaluations per pixel no longer hidden by memory traffic (`the_lattices_cost`, ignored, run by hand). That is about 15 ms for a 2 MP viewport, a large share of a frame on the integrated GPUs Apertrust targets (decision 8).

Planned engine feature: the noise computed once per viewport size and scale and cached, since it depends on the position, the coarseness and the seed only, not on the image, so dragging Strength or Mid-tones bias does not recompute it. It is measured first on an integrated GPU, the Radeon 780M. The preview keeps darktable's 21 positions, and the export, at scale 1, is unaffected.

| Case | Module effect (mean ΔE) | ΔE2000 mean | p95 | max | Pixels over 2.3 | |
| --- | --- | --- | --- | --- | --- | --- |
| `grain` | 5.559 | 0.0006 | 0.0018 | 0.0952 | 0 | pass |
| `grain-fine` | 2.132 | 0.0007 | 0.0020 | 0.0295 | 0 | pass |
| `grain-channel` | 3.275 | 0.0008 | 0.0025 | 0.0588 | 0 | pass |
| `grain-v1` | 4.124 | 0.0008 | 0.0024 | 0.0604 | 0 | pass |
| `grain-agx` | 2.907 | 0.0004 | 0.0013 | 0.0601 | 0 | pass |

## Test plan

- **Golden cases** (`crates/golden/src/cases.rs`), `disable: &[]` (no later module in either sidecar changes the image; output colour is replaced by linear Rec2020), no mask (test 0059's grain has mask mode 0):
  - `grain`: test 0059 as it is: lightness, 6400 ISO (one ulp below darktable's maximum), strength 100%, mid-tones bias 100%, offset 758.
  - `grain-fine`: test 0059's sidecar, the grain entry's params edited: 100 ISO, near the finest grain, where the noise's positions reach furthest, strength 37.3%, bias 52.7%.
  - `grain-channel`: the hue channel, which darktable ignores, 2300 ISO, strength 52%, bias 11.5%.
  - `grain-v1`: params version 1, the RGB channel, 1150 ISO, strength 64.5%, which darktable migrates with bias 0.
  - `grain-agx`: test 0186's sidecar for hlrecovery.arw with a grain entry added after AgX, as Apertrust runs it: another image, so offset 150 and a 3:2 frame 5492 wide, 3200 ISO, strength 71.3%, bias 83%.
  - Together: grain from 100 to 6400 ISO, strengths from 37.3% to 100%, biases from 0 to 100, both params versions, three of darktable's channels, two offsets, scene-referred and display-referred input.
- **Tolerance:** `Tolerance::DARKTABLE` (mean ΔE2000 ≤ 0.767, max ≤ 2.3) for every case.
- **Invariants:**
  - On the GPU (`crates/golden/tests/grain.rs`, run in CI, one `GpuContext` shared through `OnceLock`): the kernel follows a transcription of darktable's `process()` (its noise in f64, its table and lookup in f32) on a gradient image with channels below 0 and above 1, for every case's settings and darktable's defaults, the finest and coarsest grain, no and full bias, and the largest offset, within 0.15 L everywhere and 0.005 L on average (measured: at most 0.074 and 0.0023); a and b stay within 2e-3 and alpha passes through; offsets of 1 to 1 000 000 start the noise where darktable does; strength 0 moves lightness by less than 2e-3 L; output stays finite for 0, 1e-30, ±1e6 and negative input at the extreme settings; a tiled export (after exposure, at its identity, in tiles of 37×33, 64×1000 and 1000×40 over 150×101, at the operation's provisional offset) equals the whole one bit for bit, and the whole one the kernel's own output; the viewport at scales 0.5 and 0.3 (400×300 into 200×150 and 120×90) gives the transcription's `process()` at that scale, its lattice included, for every case at the provisional offset (measured: at most 0.004 L), and the kernel at those scales, through `dispatch_with`, gives it at every case's darktable offset (at most 0.0034 L); the derivatives follow the float64 model per pixel, at full resolution and through the lattice (Gradients); moving the coarseness's slot leaves the offset. An ignored test, `the_lattices_cost`, times the lattice (Results).
  - On the CPU (`setup.rs`, `lib.rs`): darktable's hash of the test image, 758, and its signed `char`; the zoom is darktable's fast-math product; the offsets lie within the period and equal the f64 term less whole periods; the table is darktable's `perm`; values that are not finite or lie outside darktable's ranges are `NotPorted`; the uniform's layout; a gradient slot moves its parameter alone; the schema's defaults are darktable's and every parameter reaches the settings; the schema has no offset, and the operation renders at the provisional one; the manifest agrees with the kernel about derivatives; `prepare` binds the permutation.
  - In `ports.rs`: test 0059's params read as darktable stores them, with offset 758; version 1 migrates with no bias; every channel reads the same; every case has kernel inputs and its slots, each moving the kernel's inputs.
  - In `crates/color`: Lab from and to XYZ D50 round-trip, and D50 is white.

## Changes after the reference release

`git log $DT_TAG..upstream/master -- src/iop/grain.c`, upstream/master at 37d514c78c, 2026-10-14: 0286264e16 ("grain: Code reformatting", line breaks and `const`) and 39df8424e6 (an unused `<assert.h>` removed). Neither changes darktable's output.
