# Port spec: bloom

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/bloom.c` `commit_params()` and `process()`, with the box mean from `src/common/box_filters.cc` and `box_filters.h` (`dt_box_mean`, `_box_mean`, `_blur_horizontal`, `_blur_vertical`, `BOX_ITERATIONS`), and the working space's conversions around a Lab module from `src/common/iop_profile.c` (`_transform_rgb_to_lab_matrix`, `_transform_lab_to_rgb_matrix`) and `src/common/colorspaces_inline_conversions.h` (`dt_XYZ_to_Lab`, `dt_Lab_to_XYZ`). The OpenCL path is `process_cl` with `data/kernels/bloom.cl`. The reference is darktable's CPU path.

Status: ported, in Slang (`crates/op/bloom`), as an operation with its manifest, schema and `prepare` (`Bloom`): every parameter, darktable's one algorithm; nothing is `NotPorted`. The box mean is shared with soften, in the colour library (`crates/color/slang/box_filter.slang` and `crates/color/src/box_filter.rs`). It is in `OPERATIONS` with `default_enabled` off, as darktable has bloom off until the user turns it on (decision 28). Checked on one GPU so far.

Golden result (case `bloom`, darktable test 0050-bloom, mire1.cr2, 3908×2602, Apple M4, a one-instance plan of the engine): ΔE2000 mean 0.0000, p95 0.0001, max 0.0003, no pixel over 2.3, against a module effect of mean 14.76, p95 27.65, max 78.87. Bloom declares no gradient slot (decision 11): the strength's derivative, which `main` carries with the pixels past the threshold held, is checked in GPU tests where no pixel crosses the threshold (Gradients).

## Purpose

The Orton effect for the photographer ("apply Orton effect for a dreamy ethereal look", darktable's description): the bright parts of the image glow into their surroundings, a soft, luminous haze around highlights.

Algorithmically, the Lab lightness is scaled by the strength and every pixel below the threshold dropped, the remaining lights are blurred by eight passes of a box mean as wide as the size, and the blur is screen-blended onto the lightness. It is the Effect tab's Bloom module (`docs/modules/README.md`), on layers too.

## Pipeline

- **darktable position:** 61.0 in the v5.0 order (`v50_order`), after colorzones (60.0), before colorize (62.0), lowlight (63.0), monochrome (64.0) and grain (65.0); the same in v3.0. A creative, display-referred module after the tone mappers.
- **Apertrust slot:** display 61.0, darktable's place and number: after local contrast (display 54), before grain (display 65) (decision 22; `docs/engine.md`, Order).
- **Domain:** display-referred. darktable's description: "non-linear, Lab, display-referred" in, out and in processing; "creative" purpose. Test 0050 has no tone mapper, so there it runs on scene-referred values.
- **Input and output:** darktable's bloom is a Lab module (`default_colorspace` returns `IOP_CS_LAB`): its pipe converts the working space to Lab before it, with the working profile's matrix to XYZ D50 and `dt_XYZ_to_Lab`, and back after it. Apertrust's pipeline stays in working RGB, linear Rec2020, so the kernel makes both conversions itself, with the same matrices (`WorkingProfile::linear_rec2020`) and darktable's Lab formulas (`xyz_to_lab_d50` and `lab_to_xyz_d50` in the colour library), as local contrast and grain do. Only L changes; a and b stay, and alpha passes through.
- **Assumes earlier modules:** none. Lightness only.
- **Layer support:** yes. The whole-image passes see the operation's whole input, as darktable's module processes its whole region before blending; a layer's mask (Phase 4) blends the output.

## Operation

| Manifest field | Value |
| --- | --- |
| `id` | `bloom` |
| `version` | 1 |
| `slot` | display, 61.0 |
| `domain` | display |
| `layers` | true |
| `differentiable` | true: `bloom_backward` and `bloom_forward_diff`, the derivatives of `main`, the pass that writes the output (Gradients) |
| `radius` | 0: the whole-image passes see the whole image (`PassKind::Reduction`, decision 15's whole-image kind under its code name), and `main` reads only its own pixel and scratch |
| `default_enabled` | false (decision 28) |
| `provenance` | darktable, commit 20891f6c6f; `src/iop/bloom.c`, `src/common/box_filters.cc`, `src/common/iop_profile.c`, `src/common/colorspaces_inline_conversions.h` |

`tiles_whole_image_passes` is true: the one pass that reads the input, `threshold`, writes exactly the pixels of the tile it runs in, at their place in the whole image (Neighbourhood and scale).

## Parameters

darktable params version 1, 12 bytes: `size`, `threshold`, `strength` (floats). The schema (version 1) has darktable's field names, its introspection ranges and defaults; `Settings::default` holds them, and a test checks the schema against it.

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| `size` | `size` | float | 0 to 100 | 20 | "the size of bloom", in percent: the box mean's radius, `⌊2.56 · min(100, size + 1)⌋` pixels at full resolution, 2 to 256 | Bloom: Size |
| `threshold` | `threshold` | float | 0 to 100 | 90 | "the threshold of light", in Lab lightness: the scaled lightness a pixel must exceed to bloom | Bloom: Threshold |
| `strength` | `strength` | float | 0 to 100 | 25 | "the strength of bloom", in percent: the lights are the lightness times `2^(min(100, strength + 1) / 100)`, 1.007 to 2 | Bloom: Strength |

darktable's sliders show all three as percentages, over the same ranges.

## Parameter versions

| Version | Change | Migration |
| --- | --- | --- |
| 1 | `size`, `threshold`, `strength`, 12 bytes | current; darktable has no `legacy_params` for bloom |

Test 0050 is version 1. `ports.rs` reads it and refuses other versions and sizes.

## Setup on the CPU (`commit_params`, the start of `process`)

`commit_params` copies the parameters. `process` derives, before its loops, with `s` darktable's `roi_in->scale / piece->iscale`, 1 in an export:

1. **The radius at full resolution:** `rad = (int)(256.0f * (fmin(100.0f, size + 1.0f) / 100.0f))`. `fmin` is C's double function, so `size + 1` is an f32 sum and the rest is double, truncated: 53 at the default 20, 104 at test 0050's 40, 256 from 99 up, 2 at 0.
2. **The radius at the pipe's scale:** `radius = MIN(256, ceilf(rad · s))`, and the box mean's half-width `hr = (2 · radius + 1) / 2`, which is `radius`.
3. **The scale of the lights:** `scale = 1.0f / exp2f(-1.0f * (fmin(100.0f, strength + 1.0f) / 100.0f))`, the exponent in double, rounded to f32 for `exp2f`: `2^0.26` at the default 25, `2^0.71` (1.6359) at test 0050's 70, `2^0.01` at 0 and 2 from 99 up.
4. **The threshold**, as it is.

`prepare` (`crates/op/bloom/src/setup.rs`) computes the same, the double steps in f64 and `exp2f` through Rust's `f32::exp2`, the same libm call on the reference build; it scales the radius as decision 15 says (Neighbourhood and scale).

## Per pixel and over the image (`process`)

On Lab, `in` the module's input, `L` its lightness:

1. **The lights** (`process`, the first loop): `l = L · scale`, and `lights = l > threshold ? l : 0`, one float per pixel.
2. **The blur** (`dt_box_mean(lights, height, width, 1, hr, BOX_ITERATIONS)`, `BOX_ITERATIONS` = 8): eight times, every row, then every column, each value replaced by the mean of the values within `hr` of it along the line that lie inside the image. darktable's `_blur_horizontal` and `_blur_vertical` keep a running sum in f32, without Kahan summation for one channel: the left half of the window added first, then for each position the value entering added and stored as `sum / hits`, `hits` counting the values in the window; in the bulk the value leaving subtracted, then the one entering added; at the right end only subtracted, `hits` falling. The values leaving are read from a copy of the line, since the line is overwritten in place. The window is clipped at the image's edges, never padded, so a constant stays constant up to the edge. A radius of 0 is the identity.
3. **The screen blend** (the second loop): `L' = 100 − ((100 − L) · (100 − blur)) / 100`; a, b and alpha copied.

So a pixel's lightness rises by `blur · (100 − L) / 100`: by the blurred lights where the pixel is black, by nothing where it is at 100. Above 100, which only scene-referred input reaches, the same term lowers it.

**In the kernel** (`crates/op/bloom/slang/bloom.slang`): `threshold` writes the lights of the input's pixels into scratch A, laid out as the whole image at the pipe's scale, at each pixel's place in it; `box_mean` runs darktable's line walk (`box_mean_line` in the shared `box_filter.slang`), one invocation per row or column, from one scratch buffer into the other, A to B along the rows and B to A along the columns, eight times; `main` converts the pixel to Lab, reads the blur from A at its place, blends, and converts back.

## GPU plan

Every pass is `[numthreads(16, 16, 1)]`. Scratch: two buffers of the whole image at the pipe's scale, one f32 a pixel each, A (scratch 0) and B (scratch 1); binding 10 is the buffer a pass reads, 11 the one it writes.

| Pass | Kind | Size | Reads | Writes |
| --- | --- | --- | --- | --- |
| `threshold` | whole-image | the operation's input, one invocation a pixel | the pixel's L | the lights at its place, into A |
| `box_mean` along the rows, ×8 | whole-image | one invocation a row | A | B |
| `box_mean` along the columns, ×8 | whole-image | one invocation a column | B | A |
| `main` | per pixel | the image | the pixel, A at its place | the pixel |

18 passes in all, one run (decision 15): the pass that reads the input, then sixteen that read only scratch. Each line pass walks its line as darktable's loop does, so its sums are darktable's, in darktable's order. The passes' uniforms hold only what they read (decision 15): the box passes the image's size, the radius and their axis; `threshold` the size, the scale of the lights, the threshold and the conversion to Lab; `main` the size, the strength (for its derivatives) and both conversions. So a change of the threshold or the strength leaves the box passes' bytes alone, and a change of the size leaves `threshold`'s and `main`'s (a unit test).

## Neighbourhood and scale

- **Neighbourhood radius or reach** (decision 13): eight box means of radius `r` reach `8r` pixels each way, 832 at test 0050's 104 and 2048 at the largest size; as a blur, a standard deviation of `√(8 · r(r + 1) / 3)` per axis, 171 pixels at 104. In practice the whole image. So bloom's blur is whole-image passes and its reach is 0 (`Operation::reach`'s default from the manifest's radius 0): `main` reads only its own pixel and scratch. A reach from darktable's `tiling_callback` overlap, `5 · radius`, would be short of the box means' `8 · radius`, so tiles grown by it would not equal the whole image, and a reach of the true `8 · radius`, up to 2048 pixels, would leave little to tile; the whole-image passes are exact at any tile size, and take the first round of tiles instead.
- **Scale dependence:** darktable scales the radius with the pipe, `ceilf(rad · roi_in→scale / piece→iscale)`, so its preview gets a smaller blur in proportion. In Apertrust the radii follow the pipe's scale from darktable's `rad`, truncated once at full resolution (decision 15), by decision 63's rule for an iterated box mean, as soften's: each of the eight iterations takes `r` or `r + 1`, the larger first, so that their blur's variance, `Σ r_i (r_i + 1) / 3`, is the export's times the scale squared (`apertrust_color::box_filter::iterated_radii`); each at most 256, the half-width `hr = (2 r + 1) / 2` each iteration's radius as darktable derives it, and each iteration's row pass and column pass take its radius in their own uniforms. At scale 1, an export, its tiles and the 100% view, every pass is darktable's radius exactly. On mire1 for test 0050's 104: five of 64 and three of 63 at 2400 wide (scale 0.614), where darktable's ceiling gives 64 in every pass; six of 52 and two of 51 at a half; five of 26 and three of 25 at a quarter (976 wide); four of 13 and four of 12 at an eighth (488 wide), where darktable's gives 13. Bloom's default of 53 is within 1.0 % of the export's σ at an eighth, and its smallest, 2, exact at a half, where the nearest radius in every pass would be 12 % and 15.5 % too wide.
- **What Apertrust computes at each scale** (decision 15; principle 2): the lights and their blur over the whole image at the pipe's scale in every pipe, never a visible region. darktable's darkroom runs the module on the visible region at the zoom's scale, its box means clipped at that region's edges, so its view of bloom near the edges of a zoomed view differs from its export; Apertrust's does not.
- **Tile overlap needed:** none for `main`, which reads only its pixel and scratch. darktable's own tiling (`tiling_callback`) asks for an overlap of `5 · radius`, "a guess", less than the box means' `8 · radius`, so darktable's tiled result can differ from its untiled one near tile edges; it tiles only when memory runs short, and the golden renders do not tile.
- **Tiling in Apertrust:** bloom opts in to a first round of tiles (`Operation::tiles_whole_image_passes`, decision 15). `threshold`, the one pass that reads the input, writes the lights of the tile's own pixels at their place in the whole image, so the tiles together write what one pass over the whole image would, every value by the same arithmetic; the first round's tiles overlap by a pixel to the right and below, where two tiles write the same value. The box passes then run once, in the round's last tile, over the whole scratch, and the second round's `main` reads it. `a_tiled_export_with_bloom_equals_the_whole_one` (`crates/engine/tests/tiles.rs`) holds a tiled export bit-identical to the whole one, in two rounds.
- **lavapipe's 128 MiB binding:** each scratch buffer binds whole, 4 bytes a pixel of the whole image, so above about 33.5 MP no tile plan fits lavapipe's binding and the export is refused by `auto_tile` with `ExportError::NoTiles`. Decision 15's strips cannot lift it: the row passes and `threshold` could bind windows of the scratch, but a column pass's running sum walks its whole column in darktable's order, and a strip of rows would cut that sum. The refusal is accepted, an exception on decision 7's list. The remedy is transposed line passes: each column a row after a block transpose, walked whole in row strips with darktable's sums.
- **Region-of-interest changes:** none (`modify_roi_in` and `_out` are darktable's defaults).

## Precision and special values

- **fp64 in darktable:** the radius and the scale of the lights go through `fmin`, a double function, as Setup says; `prepare` computes them in f64 where darktable does. The radius is truncated, so only a size whose `2.56 · (size + 1)` lies within rounding of an integer could tell f64 from f32 apart; those sizes give exact integers both ways.
- **`-ffast-math`:** darktable's release builds compile with it, and `box_filters.cc` asks for it again by pragma. It may turn the box mean's `sum / hits`, whose divisor is constant along the bulk of a line, into a multiplication by its reciprocal, and the blend's `/ 100.0f` into `* 0.01f`, each a last-bit difference; the kernel divides, as the source reads. Nothing magnifies these bits, as grain's offset does, so the case cannot tell them apart.
- **The running sums** accumulate f32 rounding along each line, darktable's as much as ours, since the order is the same: up to 3908 terms on mire1's rows.
- **The threshold** is a hard step: a pixel whose scaled lightness the GPU's Lab conversion rounds to the other side of it from darktable's adds or removes a light of at least the threshold's value, spread by the box means over some 10⁵ pixels at test 0050's radius, a change of the blur in the fourth decimal. The case's max of 0.0003 says no such pixel matters there.
- **NaN and infinity:** darktable checks for neither. The kernel relies on neither: `hits` is at least 1, Lab's cube root takes negative input through its linear segment, and nothing else divides.
- **A guard, not darktable's: each light is bounded at 10⁶** (`MAX_LIGHT` in `threshold`, the operation's boundary, as AGENTS.md's kernel rules ask). The box passes keep running sums along whole lines, so one infinite light would make every later value of its row infinite, and the column passes, subtracting it again, NaN over the whole image, as darktable's own running sums would. A lightness of 10⁶ is far past any image: linear RGB of 10⁶ is lightness about 1.2 · 10⁴, so the guard changes no finite input a photo has, the golden case's included. An infinite pixel blooms into its neighbours and leaves every other pixel finite (`an_infinite_pixel_leaves_the_rest_of_the_image_finite`).
- **Lightness above 100 and below 0:** darktable clamps neither, and neither does the kernel: lights above the threshold blur whatever their value, and the blend lowers a lightness above 100 by `(L − 100) · blur / 100`.

## CPU and OpenCL differences in darktable

- **The box mean on OpenCL** (`bloom_hblur`, `bloom_vblur`) sums each window directly from local memory and divides by `2 · radius + 1` always, reading the image clamped to its edges (`sampleri`, clamp to edge): at the edges it weights the edge pixel more where the CPU clips the window, so darktable's GPU result differs from its CPU result within `8 · radius` of the borders.
- **A radius of 0** skips the blur on OpenCL; on the CPU the box mean of radius 0 is the identity, the same result.
- The port follows the CPU path; the references are rendered on the CPU.

## Deliberate differences

- **The blur's arithmetic is darktable's, but for `-ffast-math`'s reciprocals** (Precision), a last bit.
- **The radii at a pipe's scale below 1 match the export's variance** (decision 63), `r` and `r + 1` across the eight iterations, from darktable's full-resolution radius (decision 15), where darktable's own preview rounds `rad · s` up in every pass: not a departure for exports, where the scale is 1 and every pass is darktable's radius.
- **The whole image at every pipe's scale** (Neighbourhood and scale), where darktable's darkroom processes the visible region: principle 2's same processing at each scale, not a departure from darktable's export.
- **The strength's derivative is written out** (Gradients), which changes no value.
- **Each light is bounded at 10⁶** (Precision), a guard against an infinite pixel that changes no value an image reaches.

## Choices

1. **The box mean in the colour library** (`crates/color/slang/box_filter.slang`, `import box_filter;`, and its Rust restatement `crates/color/src/box_filter.rs`), shared, since soften runs the same `dt_box_mean` on four channels. `box_mean_line` is generic over the value type (f32 for bloom, float4 for soften) and over the line's storage (`IBoxLine`), so each kernel says where its lines live and the walk stays one copy.

   The colour crate holds shared numerical helpers beside colour maths (darktable's curve interpolation), and `tools/build-kernels.sh` puts its `slang/` on every kernel's import path. Not the plugin crate, whose Slang is the pass interface, not image maths, nor a filters crate of its own: a helper joins an existing shared crate once its second port is scheduled (decision 65). darktable's box min and max (`dt_box_min`, `dt_box_max`), which haze removal uses, are not ported here: bloom does not use them.
2. **Two image-shaped scratch buffers, ping-ponged**, so no pass reads what it writes: darktable's in-place walk keeps a copy of the values leaving the window in a per-thread buffer, which on the GPU would be a local array written at a run-time index (decision 10). 8 bytes a pixel: 81 MB for mire1, 488 MB at 61 MP.
3. **One invocation per line**, darktable's running sum: 2602 invocations along mire1's rows and 3908 along its columns, each walking its line. Not a direct sum of the `2r + 1` values at each pixel, as darktable's OpenCL does: that is parallel per pixel, but 209 reads a value for test 0050's radius and another order of summation.
4. **`threshold` as a pass of its own**, rather than folded into the first row pass, so it owns its pixels per tile and the export can take the first round of tiles.

## Tool binding notes

In the first pass (decision 2) the Bloom module is darktable's and owns every parameter (`docs/modules/README.md`): Size, Threshold and Strength, under darktable's labels and in its order, each shown as a percentage. darktable has no presets for bloom.

- **What is an identity:** no setting is. The lights are the lightness scaled by at least `2^0.01`, so even at strength 0 and threshold 100 a pixel above L 99.3 blooms, and every pixel above 100. The operation leaves a pixel unchanged only where no pixel within `8 · radius` of it reaches the threshold. So the Bloom module is switched (decision 32): untouched while off, its controls at darktable's 20 %, 90 % and 25 %, any change turning it on with them and its reset turning it off. An off module looks as any other, with no dimming and no power button.
- **Derivatives:** bloom declares no gradient slot (decision 11). Component 0 of its derivative entry points carries `main`'s derivative with respect to the strength with the pixels past the threshold held, which is not the whole render's (Gradients).

## Gradients

The derivative entry points are those of `main`, the pass that writes the output, over the scratch the forward render left (decision 15). Under decision 11 an operation's gradient slots are exact, equal to finite differences of whole renders, or not declared; bloom declares none, since no parameter's derivative from `main` is the whole render's. Component 0 carries the strength's, as follows; the other components are 0.

- **The strength** scales the lights, and the box mean is linear, so with the pixels above the threshold held, the blur's derivative is `blur · ln 2 / 100` while `strength + 1 < 100`, and 0 above. `main` applies it through `lights`, which returns the blur unchanged and carries that derivative (`[ForwardDerivative]`, `[BackwardDerivative]`). It is the derivative along the strength with the threshold moving in proportion to the lights' scale, which keeps the pixels past it. Finite differences of the strength alone also see the pixels whose scaled lightness crosses the threshold as the strength moves, each adding a light of the threshold's value: a derivative of the whole render that the hard step concentrates on those pixels, which `main` cannot see.
- **The threshold** moves only which pixels cross it, so its derivative is all crossings: 0 in `main`. A derivative pass that box-means a smoothed indicator of the pixels near the threshold, through the same sixteen passes, would give both the threshold's derivative and the crossings' share of the strength's, 74% of it on test 0050. It is built only if reference matching needs bloom.
- **The size** sets an integer radius: piecewise constant, its derivative 0.

So the case names no slot (`ports.rs`), as decision 11 asks of a derivative that is not exact (a slot is exact or not declared), and as local contrast's local laplacian filter names none. Planned: a small change to bloom's manifest, so that it too declares no derivative slots. On test 0050 the crossings make the strength's derivative of whole renders 3.8 times `main`'s, the same at every step from 1e-3 to 0.1, so the gap is real, not rounding; the harness's own step and its sweep-blind list cannot close it. The synthetic sweep passes at a step of 1e-3 only because none of its pixels happens to cross (Results).

The derivative entry points are checked where no pixel crosses, in `crates/golden/tests/bloom.rs`, run in CI: against central differences of whole renders on an image with no lightness near the threshold, and against those with the threshold moved in proportion on one with lightness everywhere; and forward against reverse mode.

Results (Apple M4, the sum of output RGB as the loss):

| Image | Reverse | Finite differences | Relative error | Forward vs reverse, per pixel |
| --- | --- | --- | --- | --- |
| Test 0050, the threshold in proportion (step 1e-2; `the_cases_gap_is_the_pixels_crossing_the_threshold`, ignored, run by hand) | 6.5202e4 | 6.5298e4 | 1.5e-3 | |
| Test 0050, the strength alone (`gradient bloom` with the slot named, steps 1e-3, 1e-2 and 0.1) | 6.5202e4 | 2.4469e5, 2.4682e5, 2.4700e5 | 0.73 to 0.74 | 8.9e-8 |
| The synthetic sweep, the strength alone (`gradient bloom --synthetic` with the slot named, steps 1e-3, 3e-3, 1e-2, 0.1) | 886.73 | 879.29, 891.54, 1014.0, 1033.7 | 8.4e-3, 5.4e-3, 0.13, 0.14 | 9.0e-8 |
| 61×47 patches, no lightness near the threshold (step 1e-2) | 23.785 | 23.759 | 1.1e-3 | below 1e-4 |
| 128×96 ramp, the threshold in proportion (step 1e-2) | 60.503 | 60.609 | 1.8e-3 | below 1e-4 |
| 128×96 ramp, the strength alone (step 0.5) | 60.503 | 90.298 | 0.49 | |

Known non-smooth spots: the threshold's step, the radius's integer steps, the strength's clamp at 99.

## Results

**Golden** (Apple M4, a one-instance plan of the engine): `bloom`, ΔE2000 mean 0.0000, p95 0.0001, max 0.0003, no pixel over 2.3. The module's effect, darktable's input against its output, is a mean of 14.76, p95 27.65, max 78.87.

**Speed** (Apple M4, a render of the plan, every pass, GPU timestamps, 3908×2602): 44.6 to 45.9 ms a render, the medians of three runs (minimums 34.9 to 36.1 ms), of which, warm, the sixteen box passes take 25 to 27 ms, about 1.6 ms each, and `threshold` 2.5 ms. The line passes are latency-bound: 2602 or 3908 invocations, each walking its line's running sum in order (Choices, 3).

**On the GPU, needing no darktable** (`crates/golden/tests/bloom.rs`): the operation equals its restatement on the CPU (the threshold, `box_mean`, the blend) within 3.1e-6 of each pixel's largest channel at radii 2, 10 and 79 on a 61×47 image, so the Slang box mean is darktable's on the CPU to f32 rounding; lights below the threshold leave the image within 8.3e-7. In `crates/engine/tests/tiles.rs`, a tiled export through the default pipeline and bloom, darktable's defaults and a radius of 2, brightens some pixel's RGB by 0.93 and 1.03 against the export without bloom, where the Lab round trip alone moves it by about 1e-6, and equals the whole one bit for bit at every tile size, in two rounds.

**The viewport against the export** (decision 15's target, `the_viewport_is_within_decision_80s_target_of_the_export`, ignored, run by hand): on the case's input, the operation on the fitted image against the export downscaled by the viewport's box filter, mean ΔE2000. Bloom has no version without its whole-image passes, as the tone equalizer has its unfiltered mask, so the baseline is decision 15 (4)'s, the per-pixel gap left once the whole-image result is exact: the viewport's blend with the export's blur, downscaled by the same filter. The share of the lights and the blur, the difference, is what the target bounds.

Measured on the Apple M4 with decision 63's radii:

| Scale | Radii | Viewport against the export | Baseline: the export's blur | Share of the lights and the blur | Target |
| --- | --- | --- | --- | --- | --- |
| 2400×1598 | 64 × 5, 63 × 3 | 0.1398 (p95 0.62, max 7.1) | 0.0418 | 0.0981 | 0.15 |
| 1/2, 1954×1301 | 52 × 6, 51 × 2 | 0.1988 (p95 0.88, max 7.1) | 0.0592 | 0.1396 | 0.15 |
| 1/4, 976×650 | 26 × 5, 25 × 3 | 0.3502 (p95 1.41, max 5.7) | 0.1056 | 0.2445 | 0.4 |
| 1/8, 488×325 | 13 × 4, 12 × 4 | 0.5038 (p95 1.80, max 5.5) | 0.1602 | 0.3435 | 0.4 |

Every scale meets the target; 1/2 has the least margin, 0.140 against 0.15. The share is almost all the lights': the blur's own part of it is 0.006, 0.000, 0.001 and 0.005 at the four scales.

The viewport thresholds each downscaled pixel, where the export thresholds its full-resolution pixels before the footprint averages them. So wherever a footprint holds pixels on both sides of the threshold, which mire1's chart has along every edge, the lights differ, and Lab's lightness of a footprint's mean is not the mean of its lightnesses.

## Test plan

- **Golden case:** `bloom`, darktable's test 0050-bloom on mire1.cr2, unchanged: size 40, threshold 85, strength 70, after darktable's default pipeline without a tone mapper. `Tolerance::DARKTABLE`, a one-instance plan of the engine (decision 15).
- **Unit tests** (`crates/op/bloom`): the schema's defaults are darktable's and its values round-trip; every parameter reaches its setting; the radius and the scale of the lights are darktable's at full resolution (53, 104, 256, 2; `2^0.71`) and the radii follow the pipe's scale by decision 63, each iteration's row and column pass taking its radius; `prepare`'s passes, sizes, bindings and scratch, every pass within eight storage bindings, only `main` per pixel and writing the output; the uniforms each pass holds (a change of the threshold or the strength leaves the box passes alone, of the size `threshold` and `main`); the uniforms' layout; in a tile, the scratch and the box passes cover the whole image; the manifest says the operation is differentiable, and the operations' own test that its kernel agrees.
- **The shared box mean** (`crates/color`): a constant line stays constant; each line equals the mean over its clipped window, computed in f64, within f32 rounding, for radii from 0 past the line's length; a radius of 0 is the identity bit for bit. Each iteration's radius (`iterated_radii`, decision 63): its worked values, test 0050's 104 at the four scales among them; every pass darktable's radius at scale 1 for every radius to 2048; each radius `r` or `r + 1`, the larger first, the variance within half a step.
- **In `ports.rs`:** test 0050 reads as version 1 with size 40, threshold 85 and strength 70, and names no gradient slot (Gradients); other versions and sizes are refused.
- **On the GPU** (`crates/golden/tests/bloom.rs`, run in CI): the operation equals its restatement on the CPU (the threshold, `apertrust_color::box_filter::box_mean`, the blend) on a 61×47 image of patches with chroma, at radii of 2, 10 and 79, the last wider than the image, which checks the Slang box mean against its Rust twin; alpha passes through; lights below the threshold leave the image unchanged, up to the Lab round trip; output stays finite for black, 1e-30, negative, very bright (10⁶) and saturated input at the settings' ends, and an infinite pixel leaves every other pixel finite (the guard, Precision); the strength's derivative against central differences of whole renders where no pixel crosses the threshold, and forward against reverse mode (Gradients). And, ignored, run by hand: the case's gradient gap and the viewport's target (Results). In `crates/engine/tests/tiles.rs`, run in CI: a tiled export through the default pipeline and bloom, at darktable's defaults and at a radius of 2, brightens some pixel's RGB by more than 0.1 against the export without bloom, so something blooms beyond the Lab round trip's rounding, and equals the whole one bit for bit at every tile size, in two rounds.
