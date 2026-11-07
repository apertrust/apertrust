# Port spec: vignetting

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/vignette.c` `commit_params()` and `process()`, with `encrypt_tea` and `tpdf` from `src/common/tea.h` for its dithering. The OpenCL kernel is `vignette` in `data/kernels/extended.cl` (program 8 in `programs.conf`). The reference is darktable's CPU path.

Status: **ported**, without dithering.

- **What:** every continuous parameter (fall-off start and radius, brightness, saturation, centre, width-to-height ratio, shape), the automatic ratio and clipping (`unbound`), in Slang (`crates/op/vignette`), with its manifest and schema.
- **In the pipeline:** in `OPERATIONS` with `default_enabled` off. An unedited photo does not run it; the session plans it only while a module's binding sets one of its parameters (decision 28), as the Vignetting module's does (`docs/modules/vignetting.md`).
- **Not ported:** dithering. `prepare` returns `NotPorted` for 8-bit and 16-bit dithering, whose noise darktable draws from a sequence that depends on its thread count, and with which darktable's CPU path also switches its fall-off from linear to a raised cosine (Dithering; Deliberate differences).

**Golden result** (six cases on test 0045's mire1.cr2, 3908×2602, Apple M4, 48-byte pass header): every case passes, ΔE2000 mean below 5e-5, max 0.0002 at most (Results).

**Gradients:** every parameter the check's loss can see passes on every case and on the synthetic sweep, at worst 3.9e-3; forward and reverse mode agree within 3.9e-6. The saturation, which that loss (R + G + B) cannot see, passes a GPU test with unequal channel weights at 2.0e-5 (Gradients).

## Purpose

A vignette for the photographer: darken (or lighten) and desaturate (or saturate) the image towards its edges, to frame the subject, imitate a lens's fall-off or, with a white vignette, lift the corners.

Algorithmically, each pixel gets a weight from its distance to a centre, measured in a superellipse whose shape runs from a rectangle through an ellipse and a diamond to a star. The weight is 0 inside the inner radius and rises linearly to 1 across the fall-off. By that weight, the brightness then scales the pixel (darkening) or adds to it (lightening), and the saturation moves its channels away from or towards their mean.

It is the Effect tab's Vignette module (layers yes), display-referred, after the tone mapper (`docs/layers.md`, Scene and display domains).

## Pipeline

- **darktable position:** 68.0 in the v5.0 order, after the tone mappers (sigmoid 45.3, AgX 45.5, filmic rgb 46.0) and the other creative modules (grain 65.0, soften 66.0, split-toning 67.0), before final scale (69.4) and output colour (70.0). The v3.0 order of test 0045 has it at 68.0 too, before output colour.
- **Apertrust slot:** the display stage at 68.0, its domain display, darktable's place: after the tone mapper and grain (65), before output colour (decision 22; `docs/engine.md`, Order).
- **Domain:** display-referred. darktable's description: "non-linear, RGB, display-referred" in and out; "creative" purpose. In darktable's modern pipeline it runs on the tone mapper's output; test 0045 has no tone mapper, so there it runs on scene-referred values, some above 1.
- **Input and output:** RGBA in the working space (`IOP_CS_RGB`, linear Rec2020 by default in darktable and always in Apertrust); alpha passes through in Apertrust (CPU and OpenCL differences).
- **Assumes earlier modules:** nothing about colour. Its geometry assumes the size of its input at full resolution, darktable's `piece->buf_in`: the image after any crop or geometry module before it (Neighbourhood and scale).
- **Layer support:** yes: per pixel, no statistics of the image. Its weight depends on the pixel's position in the whole image, which the pass header gives.

## Parameters

darktable params version 4, 44 bytes: `scale`, `falloff_scale`, `brightness`, `saturation` (f32), `center` (two f32, x then y), `autoratio` (gboolean, i32), `whratio`, `shape` (f32), `dithering` (`dt_iop_dither_t`, i32), `unbound` (gboolean, i32).

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| `scale` | `scale` | float | 0 to 200 | 80 | "fall-off start": the inner radius, where the weight leaves 0, in percent of half the image (half its width across and half its height down with the automatic ratio; otherwise half its longer side, times the ratio along the narrower axis) | Vignetting |
| `falloff_scale` | `falloff_scale` | float | 0 to 200 | 50 | "fall-off radius": the fall-off's width, from weight 0 to 1, in the same units; at least `100 / min(width, height)`, about a pixel | Vignetting |
| `brightness` | `brightness` | float | −1 to 1 | −0.5 | Strength on brightness: below 0 scales the pixel by `1 + weight · brightness`, from 0 adds `weight · brightness` to every channel | Vignetting |
| `saturation` | `saturation` | float | −1 to 1 | −0.5 | Strength on saturation: scales each channel's distance from the channels' mean by `1 + weight · saturation`; below 0 desaturates, above 0 saturates | Vignetting |
| `center_x` | `center.x` | float | −1 to 1 | 0 | "horizontal center": the vignette's centre, in half the image's width from its middle, right positive | Vignetting |
| `center_y` | `center.y` | float | −1 to 1 | 0 | "vertical center": in half the image's height from its middle, down positive | Vignetting |
| `autoratio` | `autoratio` | bool | | false | "automatic ratio": the vignette's width-to-height ratio follows the image's | Vignetting |
| `whratio` | `whratio` | float | 0 to 2 | 1 | "width/height ratio", without the automatic ratio: up to 1 the width over the height, from 1 to 2 the height over the width taken from 2, `2 − h/w` | Vignetting |
| `shape` | `shape` | float | 0 to 5 | 1 | "shape": the superellipse's exponent is `2 / shape`: 0 a rectangle, 1 an ellipse, 2 a diamond, above 2 a four-pointed star | Vignetting |
| `dithering` | `dithering` | choice | `off`, `8bit`, `16bit` | `off` | Random noise against banding, at 1/256 or 1/65536; not ported | none: `NotPorted` (Dithering) |
| `unbound` | `unbound` | bool | | true | Values stay unclipped; off, every channel of a pixel with weight above 0 is clipped to [0, 1] after each step | Vignetting, fixed on (Tool binding notes) |

The defaults are darktable's introspection defaults (the module has no `init` or `reload_defaults`). darktable's one preset, "lomo": scale 40, falloff 100, brightness −1, saturation 0.5, centre 0, ratio fixed at 1, shape 1, no dithering, unbound.

## Parameter versions

| Version | Change | Migration (`legacy_params`) |
| --- | --- | --- |
| 1 | doubles: `scale`, `falloff_scale`, `strength` (percent), `uniformity`, `bsratio`, `invert_falloff`, `invert_saturation` (gboolean), `center` (two doubles) | `brightness = −(1 − max(bsratio, 0)) · strength / 100`, `saturation = −(1 + min(bsratio, 0)) · strength / 100`; `invert_saturation` multiplies saturation by −2, `invert_falloff` negates brightness; `uniformity` is dropped; automatic ratio on, ratio 1, shape 1, no dithering, clipped (`unbound` false) |
| 2 | floats: `scale`, `falloff_scale`, `brightness`, `saturation`, `center`, `autoratio`, `whratio`, `shape` | copied; no dithering, clipped |
| 3 | adds `dithering` (int) | copied; clipped |
| 4 | adds `unbound` | current |

`ports.rs` reads version 4, the only one the cases contain; older versions wait for the import of darktable's edits (decision 51, planned).

## Setup on the CPU (`commit_params`, the start of `process`)

`commit_params` copies the parameters. `process()` derives, before its loop, with `W` and `H` the input's size at full resolution (`piece->buf_in`), `s` the pipe's scale (`roi_in->scale`, equal to `roi_out->scale`) and `roi_in->x`, `roi_in->y` the region's corner at that scale:

1. **The centre**, at full resolution: `vc = (W/2 + center.x · W/2, H/2 + center.y · H/2)`. `center.x * buf_in->width` is a float times an int, so an f32 product. The halving by `2.0` and the addition are in f64 but exact, and the result is stored as f32, so `vc` is the f32 expression's value bit for bit. Then in the region: `roi_center = vc · s − (roi_in->x, roi_in->y)`.
2. **The scales** from pixels at the pipe's scale to the vignette's units:
   - with the automatic ratio, `xscale = 2 / (W s)`, `yscale = 2 / (H s)`;
   - otherwise `basis = 2 / (max(W, H) s)`, and for `whratio ≤ 1` `yscale = basis`, `xscale = basis / whratio`, else `xscale = basis`, `yscale = basis / (2 − whratio)`.
3. **The radii:** `dscale = scale / 100`; `fscale = max(falloff_scale, 100 / min(W, H)) / 100`, the minimum in f64 ("to smooth out aliasing artifacts").
4. **The exponents:** `shape' = max(shape, 0.001)`, `exp1 = 2 / shape'`, `exp2 = shape' / 2`.
5. **The scaled centre:** `roi_center_scaled = roi_center · (xscale, yscale)`.
6. **The dither amplitude:** 0, 1/256 or 1/65536.

Apertrust's `prepare` (`crates/op/vignette/src/setup.rs`) checks the settings and passes darktable's parameters to the kernel, in darktable's units. Steps 1 to 5 are the kernel's setup, `geometry()`, in the same f32 expressions, so its derivatives reach the parameters. `W`, `H` and `s` are the pass header's `full_width`, `full_height` and `scale`. The kernel works in the whole image's coordinates at the pipe's scale, so it subtracts no region corner (Neighbourhood and scale).

## Per pixel (`process`)

darktable (`vignette.c:process`), for the pixel at column `i` and row `j` of the region:

1. **Position:** `pv = (|i · xscale − roi_center_scaled.x|, |j · yscale − roi_center_scaled.y|)`.
2. **Length from the centre:** `cplen = (pv.x^exp1 + pv.y^exp1)^exp2` (`powf`).
3. **Weight:** 0 if `cplen < dscale`; otherwise `(cplen − dscale) / fscale`, clipped to [0, 1]. With dithering on, a weight strictly between 0 and 1 becomes `0.5 − cos(π · weight) / 2` and the pixel draws its noise, `dith = amplitude · tpdf(...)` (Dithering).
4. **If the weight is above 0**, on each of the four channels (`for_each_channel`, four wide):
   1. **Brightness:** for `brightness < 0`, `col = col · (1 + weight · brightness) + dith`; otherwise `col = col + weight · brightness + dith`.
   2. **Clip** to [0, 1] unless `unbound`.
   3. **Saturation:** `mv = (r + g + b) / 3`, `col = col − (mv − col) · weight · saturation`, then clip unless `unbound`.
5. Otherwise the pixel is copied.

The kernel (`slang/vignette.slang`) does the same on RGB with the global position, `X = origin_x + id.x`, `Y = origin_y + id.y`: `pv = |X · xscale − vc.x s · xscale|`, and likewise in y.

- It computes the length as `m · (1 + (n/m)^exp1)^exp2`, with `m` the larger of `pv.x` and `pv.y` and `n` the smaller, equal to darktable's within rounding but free of its overflow and underflow (Precision).
- Whether the weight is above 0 is found first, outside the differentiable function, which gets it as an argument.
- Without dithering the weight stays linear, as on darktable's CPU.
- Alpha passes through.

## Dithering

**What darktable does.** darktable's dithering adds triangular noise, `tpdf(u)` in [−1, 1] for a uniform 32-bit `u`, times 1/256 or 1/65536, to every channel of each pixel in the fall-off (weight strictly between 0 and 1), after the brightness step. Its random numbers are not deterministic across machines or tiles:

- **CPU:** each OpenMP thread keeps a TEA state (`alloc_tea_states`), whose first word is reset to `j · height` at each row (the region's row `j`, so a tile reseeds differently) and whose second word carries over from the thread's previous rows. Each pixel in the fall-off advances it once. Which rows a thread handles depends on the thread count (`schedule(static)`), so the noise differs from one machine to another.
- **OpenCL:** the kernel seeds each pixel independently, `{mad24(y, width, x), 0}`, one TEA encryption each, with `x`, `y` and `width` those of the region it renders, so its noise changes with darktable's tiles too.

**The noise alone fails test 0045.** No port can match darktable's noise pixel for pixel. Against a CPU emulation of darktable's `process()` without the noise (`0.5 − cos` weight, test 0045's parameters), darktable's render differs by exactly the noise:

- in the fall-off by at most 0.9995/256, the same on every channel within 4e-7 (with `unbound` on, the noise shifts every channel and their mean alike, so the saturation step passes it unchanged);
- outside it by at most 7e-7.

That difference is ΔE2000 mean 0.133, p95 0.77, max 2.98, with 621 pixels over 2.3: the darkest pixels of the fall-off, where a step of 1/256 in linear light is a large step in L*.

**So the port leaves dithering out.** `prepare` returns `NotPorted` for 8-bit and 16-bit dithering, and the `vignette` case is test 0045 with dithering off (Test plan). In darktable's CPU path the raised-cosine weight comes only with dithering, so the kernel, as built, leaves it out too; the raised cosine for every vignette is planned (Deliberate differences). No case could check the noise against darktable, and banding is the export's quantisation's concern, where Apertrust can dither once for the whole image if it wants to.

**Porting it later** is a small change: one TEA encryption per pixel, as darktable's OpenCL kernel does, but seeded from the pixel's position in the whole image rather than in its region, so deterministic and the same in every tile. That is statistically the same noise as darktable's but never the same pixels: TEA's eight rounds and `tpdf` in the kernel, and the raised cosine.

## Neighbourhood and scale

- **Neighbourhood radius:** none: per pixel. The manifest's radius is 0; a tile needs no border.
- **Position and scale:** the weight depends on the pixel's position relative to the input at full resolution, `W × H` (`piece->buf_in`), and scales with the pipe. `xscale = 2 / (W s)` maps a pixel at column `X` of the image at scale `s` to `X / s` at full resolution, in half-widths, and the centre is placed at `vc · s`.
  - The vignette therefore covers the same part of the image at every scale, and `fscale`'s minimum is fixed at full resolution, so the preview and the export agree.
  - The kernel takes `W`, `H` and `s` from the pass header and adds the header's origin to its invocation, so a tile, a region and the viewport compute what the whole image would at their scale (decision 13).
  - `crates/golden/tests/vignette.rs` checks a tiled export against the whole one bit for bit, and the viewport pipe at scales 0.5 and 0.3 against darktable's formula at that scale.
- **Region-of-interest changes:** none. vignette.c has no `modify_roi_in`, `modify_roi_out` or `tiling_callback`, and does not read `piece->iwidth`, `piece->iheight` or `piece->iscale`; it uses `piece->buf_in` and `roi->scale` alone. Its flags allow darktable's tiling with the full region of interest (`IOP_FLAGS_TILING_FULL_ROI`), so a darktable tile also places itself through `roi_in->x` and `roi_in->y`.
- **Full resolution in the header:** the engine fills `full_width` and `full_height` with the operation's stage at full resolution, its input's, as darktable's per-module `buf_in` is (decision 16): the developed image, or after Orientation's quarter turn the turned image, so the vignette centres and shapes itself on the photo as shown, as darktable's does. `crates/engine/tests/geometry.rs` holds the vignette after each of the seven orientations to the vignette on the image flipped beforehand, bit for bit, whole and in tiles (`a_vignette_after_orientation_frames_the_turned_image`). Crop's window will reach it the same way.

## Precision and special values

- **fp64:** darktable computes the minimum fall-off, `100.0 / MIN(W, H)` and the `MAX` with `falloff_scale`, in f64 and stores `fscale` as f32; the kernel computes it in f32, a difference of an ulp or so of `fscale`. The centre is not one: darktable's product is f32 and its f64 halving and addition are exact (Setup on the CPU), so it equals the kernel's f32 `W/2 + center.x · W / 2` bit for bit.
- **The length's underflow and overflow at small shapes:** darktable's `powf(powf(pv.x, exp1) + powf(pv.y, exp1), exp2)`, with `exp1 = 2 / shape` large, goes wrong both ways below a shape of about 0.03, inside the image.
  - **Underflow:** where both `pv.x^exp1` and `pv.y^exp1` fall below the smallest f32, the length is 0 and the pixel gets no vignette, whatever the radii. For pixels at 0.5 from the centre that starts at `exp1` = 126, a shape of 0.016; at shape 0 (taken as 0.001, `exp1` = 2000), every pixel whose `pv.x` and `pv.y` both lie below about 0.95 gets length 0.
  - **Overflow:** `pv^exp1` passes the largest f32 for `pv` above `e^(44.36 · shape)`: 1.045 at shape 0.001, 1.56 at 0.01, 2.43 at 0.02, 3.96 at 0.031. There the length is infinite and the weight 1, a full vignette, wherever the fall-off ends beyond that point (`dscale + fscale` reaches 4). At shape 0, scale 80 and fall-off 100, a pixel at `pv.x` = 1.2 gets weight 1 from darktable and 0.4 from the kernel.
  - The kernel's `m · (1 + (n/m)^exp1)^exp2` is the same length (`exp1 · exp2 = 1`), its base between 1 and 2, so it neither underflows nor overflows: at shape 0 its length is the larger of `pv.x` and `pv.y`, a rectangle (`a_rectangle_keeps_its_inner_radius`). Below a shape of about 0.03 it therefore differs from darktable through both; for shapes from 0.2 to 5 the two agree within f32 rounding.
  - That is on the M4. CI's paravirtual Metal device rounds its transcendentals differently, and there the kernel and the CPU model of darktable's process differ by up to 2.2e-5 (shape 1.49), so `crates/golden/tests/vignette.rs` allows 5e-5 and reports its worst pixel.
- **Division by zero in the ratio:** at a `whratio` of exactly 0 or 2, without the automatic ratio, darktable divides by zero and `xscale` (or `yscale`) is +inf. What follows depends on the region:
  - where `vc · s − roi.x` is positive, as for the whole image or a tile before the centre, `roi_center_scaled` is +inf and every `pv` NaN (`i · inf − inf`), every comparison false, and the pixels are left unchanged;
  - where it is negative, a darktable tile past the centre, it is −inf, `pv` is +inf but in the tile's first column (`0 · inf`, NaN), and the weight 1, a full vignette.

  Just inside either end the vignette is an extremely narrow ellipse that covers nearly the whole image. `prepare` keeps `whratio` within [0.001, 1.999], that limit from inside, the same in every tile (Deliberate differences).
- **Branches:** the weight's clip, the gate on weight above 0, the brightness's sign and `whratio ≤ 1` are darktable's; the kernel keeps them (Gradients).
- **NaN, infinity and negative values:** the weight is finite and within [0, 1] for any finite position, so finite input gives finite output; negative channels pass through every step, unclipped while `unbound`. darktable's NaN at a ratio of 0 or 2 cannot occur in the kernel.

## CPU and OpenCL differences in darktable

- **The fall-off curve:** the OpenCL kernel always turns a weight strictly between 0 and 1 into `0.5 − cos(π · weight) / 2`; the CPU only with dithering on. So with dithering off, darktable's CPU fall-off is linear and its GPU's a raised cosine; test 0045's `cpugpu.maxpix` allows 720 000 pixels to differ between the two. Before darktable 3.4 both paths always used the cosine: f9454c05e2 ("don't compute random number for dither when dithering is disabled", 2020) added an early branch for dithering off that skipped the cosine along with the random number, and c803502df4 (2023) kept that behaviour.
- **The dither noise's seeds:** per thread and row on the CPU, per pixel on OpenCL (Dithering). OpenCL also adds noise where the unclipped weight is exactly 1, the CPU only strictly between 0 and 1.
- **Alpha:** the CPU's `for_each_channel` runs over four channels, so the brightness step (and its noise) and the saturation step change alpha too; OpenCL changes only RGB. The CPU path has done so only since c803502df4 (2023), which replaced its per-channel code, alpha copied, with the four-wide loop; before, both paths kept alpha.
- Apertrust follows the CPU, and passes alpha through.

## Deliberate differences

- **The raised-cosine fall-off, planned:** the weight `0.5 − cos(π w)/2`, darktable's GPU path's and its CPU path's with dithering, for every vignette, an agreed difference from darktable's CPU path, which is linear without dithering. It is smooth at both ends of the fall-off, the better feather and smooth for the gradients. The kernel switches, and the vignette golden cases are re-rendered from darktable's OpenCL path, or become cases with 16-bit dithering, whose noise, 1/65536, is 256 times weaker than the 8-bit noise that reaches ΔE 2.98. As built, the kernel follows the CPU, linear.
- **No dithering:** `prepare` returns `NotPorted` for 8-bit and 16-bit dithering, and a test covers it. The noise cannot match darktable's, whose sequence depends on its thread count, and fails test 0045 on its own (Dithering).
- **The length from the centre** as `m · (1 + (n/m)^exp1)^exp2`: the same function, free of darktable's underflow (no vignette) and overflow (a full vignette), both of which change the image below a shape of about 0.03 (Precision).
- **The ratio kept within [0.001, 1.999]** when not automatic: at exactly 0 or 2 darktable divides by zero and leaves the image unchanged or vignettes it fully, depending on the tile; the kernel gives the limit from inside, an extremely narrow vignette.
- **The geometry computed in the kernel**, in f32, so the derivatives are with respect to darktable's own parameters; darktable computes the minimum fall-off in f64.
- **The whole image's coordinates:** `X · xscale − vc.x s · xscale` with `X` the pixel's column in the whole image at the pipe's scale, where darktable computes `i · xscale − (vc.x s − roi.x) · xscale` with `i` the column in its region: the same value, rounded differently, and the same in every tile.
- **Alpha passes through**; darktable's CPU path applies the brightness and saturation steps to it. Apertrust's layers blend in the engine, not by alpha.

## Tool binding notes

In the first pass (decision 2) the Vignetting module, built (`docs/modules/vignetting.md`), is darktable's:

- **Controls:** brightness, saturation, fall-off start and radius, centre, shape, automatic ratio and width/height ratio.
- **Defaults:** brightness and saturation start at 0, so an untouched module leaves the plan; darktable's −50 % each and its lomo preset are presets (`docs/modules/README.md`).
- **Brightness and saturation at 0 are a no-op** when `unbound` is on, darktable's default: brightness 0 and saturation 0 give every pixel back bit for bit, whatever the radii, ratio, shape and centre (`col + w · 0`, `col − (mv − col) · 0`; `crates/golden/tests/vignette.rs`).
- **Clipping stays off:** the binding owns `unbound` and fixes it on, with no control, as darktable shows none. With `unbound` off, darktable clips the vignetted pixels to [0, 1] after each step, which also makes brightness 0 change pixels outside [0, 1] beyond the inner radius. After the tone mapper Apertrust's values mostly lie within [0, 1], but not always (HDR headroom, out-of-gamut colours).

## Gradients

The derivative entry points differentiate against four of the eight continuous parameters at a time, in darktable's units, chosen by the uniform's four gradient slots, each naming one of these:

| Id | Parameter |
| --- | --- |
| 0 | `scale` |
| 1 | `falloff_scale` |
| 2 | `brightness` |
| 3 | `saturation` |
| 4 | `center_x` |
| 5 | `center_y` |
| 6 | `whratio` |
| 7 | `shape` |

- The kernel computes the geometry from them (`geometry()`, darktable's `process()` before its loop), so no chaining on the CPU is needed.
- With the automatic ratio, `whratio` stands for nothing and its gradient is 0.
- `main` reads the parameters without slots, so they cost nothing there.
- Whether a pixel lies beyond the inner radius, darktable's gate, is found first and passed in, so both derivative passes see the forward pass's decision.

### What `apertrust-golden gradient` can check

Its loss is the output's R + G + B, and the saturation step keeps the channels' mean: `Σ (col − (mv − col) w s) = Σ col`. That loss's derivative with respect to the saturation is therefore 0 wherever nothing clips, and the check compares rounding noise:

- on `vignette`, reverse −1.2e-2 against finite differences −7.0e-3 over 10 MP, per-pixel values of 1e-8 against single-ulp steps of 6e-5, a relative error of 0.40;
- on `vignette-clip`, where clipping breaks the mean, the totals agree within 1.0e-3, but forward and reverse mode differ by 3.3e-4 per pixel, the same noise against a small total.

So no case's slots name the saturation, and `crates/golden/tests/vignette.rs` checks every parameter, the saturation included, with a loss that weights the channels unequally, `0.25 r + 0.5 g + b` (below). Planned: the harness gains a loss of that kind, unequal channel weights (0.25, 0.5, 1), or a second loss per parameter, so that it checks the saturation too.

The cases' slots (`ports.rs`) cover every other parameter at least once on the test image and the sweep:

| Case | Slots |
| --- | --- |
| `vignette` | brightness, scale, fall-off, ratio |
| `vignette-autoratio` | centre x and y, shape, scale |
| `vignette-narrow` | ratio, shape, centre y, scale |
| `vignette-star` | shape, ratio, scale, fall-off |
| `vignette-clip` | brightness, fall-off |
| `vignette-white` | brightness, scale, fall-off, shape (not its centre, whose y the emulation below gives 1.4e-2 on the sweep) |

### Choosing the slots near the fall-off's edges

**The narrow case does not check the centre's x.** At shape 0.37 (`exp1` = 5.4) the vignette's sides are nearly straight columns. On the sweep its outer edge passes within 1.3e-7 of one column, x = 347, so that whole column sits on the fall-off's kink at weight 1, where autodiff takes the inner slope and central differences average it with 0: reverse mode there is twice the finite difference (reverse −63.2, finite differences −138.2, relative error 0.54, the same at steps from 1e-4 to 1e-2, forward and reverse within 8.3e-8). A step of the centre's x moves the sides by 0.18 of the sweep's pixels, and the left and right sides' contributions largely cancel in the total, −63, so any column near an edge weighs heavily.

**The slots were chosen by a float64 emulation of the check** (finite differences at the check's step against a step of 1e-6), on the sweep and under small moves of every setting. It reproduces the finite differences above, −138.2, and shows no setting near the narrow case's safe for the centre's x: settings that pass fail again when the centre or the ratio moves by 0.002.

- The centre's x is checked on the automatic-ratio case instead, whose shape 0.83 has curved sides (worst 1.5e-3 in the emulation under the same moves).
- The narrow case checks the centre's y and the inner radius (worst 2.8e-3).
- Test 0045's settings would give the centre's x 8.5e-3 on the sweep, and 6.6e-2 nearby, so `vignette` does not check it either.
- Every slot the cases have stays below 3e-3 in the emulation under those moves, test 0045's ratio at 5.3e-4.

### Gradient results

Results, Apple M4, step 1e-3, relative error of reverse mode against central finite differences per parameter, and the largest per-pixel difference between forward and reverse mode over the case's parameters:

| Case | Parameters (relative error) | Forward vs reverse |
| --- | --- | --- |
| `vignette` | brightness 1.3e-5, scale 1.4e-3, fall-off 1.5e-4, ratio 1.0e-4 | 1.8e-7 |
| `vignette-autoratio` | centre x 1.1e-4, centre y 7.4e-5, shape 1.4e-5, scale 1.3e-4 | 1.8e-7 |
| `vignette-narrow` | ratio 1.6e-4, shape 1.2e-5, centre y 1.7e-5, scale 7.9e-5 | 2.5e-7 |
| `vignette-star` | shape 5.4e-5, ratio 5.6e-5, scale 1.6e-3, fall-off 3.9e-3 | 3.9e-6 |
| `vignette-clip` | brightness 1.2e-5, fall-off 8.1e-4 | 1.9e-7 |
| `vignette-white` | brightness 1.3e-5, scale 1.3e-4, fall-off 2.0e-3, shape 2.3e-5 | 1.9e-7 |

With `--synthetic`:

| Case | Parameters (relative error) | Forward vs reverse |
| --- | --- | --- |
| `vignette` | brightness 1.3e-5, scale 1.4e-3, fall-off 1.6e-4, ratio 5.9e-4 | 1.4e-7 |
| `vignette-autoratio` | centre x 2.9e-4, centre y 1.0e-4, shape 1.2e-4, scale 1.5e-4 | 1.9e-7 |
| `vignette-narrow` | ratio 1.3e-3, shape 3.1e-5, centre y 3.2e-4, scale 2.0e-4 | 3.3e-7 |
| `vignette-star` | shape 1.0e-4, ratio 5.1e-4, scale 1.7e-3, fall-off 3.9e-3 | 2.1e-7 |
| `vignette-clip` | brightness 1.4e-5, fall-off 8.0e-4 | 2.2e-7 |
| `vignette-white` | brightness 1.1e-5, scale 1.4e-4, fall-off 1.9e-3, shape 1.7e-5 | 9.7e-8 |

The star's and the white vignette's radii are the largest: a step of 1e-3 percent moves the weights by about 1e-5, a few hundred ulps of the output, and the float64 emulation of the same check gives 1e-5 or less, so the rest is the f32 output's rounding, not a kink.

**With unequal channel weights** (`derivatives_match_finite_differences_for_every_parameter`, a 192×128 gradient image with channels below 0 and above 1, step 1e-3), on test 0045's settings and on a ratio below 1 with positive saturation (ratio 0.71, shape 0.9, fall-off from 31.3% over 112.3%, brightness −0.57, saturation +0.29, centre (−0.09, 0.21)):

| Parameter | Test 0045's settings | Ratio below 1 |
| --- | --- | --- |
| saturation | 2.0e-5 | 1.2e-5 |
| brightness | 1.3e-5 | 1.3e-5 |
| scale | 1.4e-3 | 1.6e-4 |
| fall-off | 4.9e-5 | 2.1e-4 |
| centre x | 5.8e-4 | 2.3e-4 |
| centre y | 2.3e-4 | 1.4e-4 |
| ratio | 1.5e-4 | 5.5e-5 |
| shape | 3.8e-5 | 3.4e-5 |

Forward and reverse mode agree within 1.2e-7. The test fails above 1e-2 and 1e-4.

- The second settings were chosen by a float64 emulation of this test: they stay at 5.6e-4 or below for every parameter when any setting moves (centre and ratio by 0.002, radii by 0.1, shape by 0.01). The narrow case's settings, with their near-straight sides, would put the centre's x at 5.1e-3, half the limit, which the emulation reproduces.
- Test 0045's settings give at most 5.8e-4 in the emulation; the GPU's 1.4e-3 for its scale is f32 rounding (the emulation gives 2.9e-5).

Known non-smooth spots, and what an optimiser would use instead:

- **The fall-off's edges**, where the linear weight meets 0 and 1: kinks, with slopes 0 and `1/fscale`. darktable's own raised cosine, `0.5 − cos(π w)/2`, is the smooth version: its slope is 0 at both ends (Deliberate differences).
- **The brightness at 0**: multiplicative below, additive from 0, slopes `w · col` and `w`. A single form, `col · (1 + w b⁻) + w b⁺`, is the same function; an optimiser would smooth the switch or optimise one side.
- **The ratio at 1** (`whratio ≤ 1`): continuous, the slope changes sides between `xscale` and `yscale`.
- **The centre's axes at shapes above 2**: the length has a cusp where `pv.x` or `pv.y` is 0, its derivative there infinite; Apertrust's `pow0` gives 0 exactly on the axis.
- **Clipping**, with `unbound` off: kinks at 0 and 1, and a jump at the inner radius for a channel outside [0, 1], which the gate clips just outside and leaves unclipped just inside. Moving the radii, centre, ratio or shape then moves a discontinuity across pixels, so `vignette-clip` checks only the brightness and the fall-off's width, which leave the inner radius in place.
- **The minimum fall-off and shape** (`max`): the gradient is 0 below them.

## Results

Apple M4, 48-byte pass header (decision 14's), every case `Tolerance::DARKTABLE`. The kernel reproduces darktable's CPU path within rounding: the largest difference is 0.0002 ΔE2000, on the star.

A single pass takes 3.6 to 4.5 ms over 10 MP (exposure's 3.9 ms: memory-bound); with `--repeat 32`, 0.17 to 0.21 ms per application of the per-pixel function.

| Case | Module effect (mean ΔE) | ΔE2000 mean | p95 | max | Pixels over 2.3 | |
| --- | --- | --- | --- | --- | --- | --- |
| `vignette` | 14.18 | 0.0000 | 0.0000 | 0.0001 | 0 | pass |
| `vignette-autoratio` | 7.08 | 0.0000 | 0.0000 | 0.0001 | 0 | pass |
| `vignette-narrow` | 7.37 | 0.0000 | 0.0000 | 0.0001 | 0 | pass |
| `vignette-star` | 21.11 | 0.0000 | 0.0000 | 0.0002 | 0 | pass |
| `vignette-clip` | 15.10 | 0.0000 | 0.0000 | 0.0001 | 0 | pass |
| `vignette-white` | 20.65 | 0.0000 | 0.0001 | 0.0002 | 0 | pass |

Test 0045 as it stands, with 8-bit dithering, is not a case: against a CPU emulation of darktable without the noise its render differs by the noise alone, at most 0.9995/256 per channel, ΔE2000 mean 0.133, p95 0.77, max 2.98, 621 pixels over 2.3 (Dithering).

## Test plan

- **Golden cases** (`crates/golden/src/cases.rs`), all on mire1.cr2, `disable: &[]` (no later module in test 0045's sidecar changes the image; output colour is replaced by linear Rec2020), no mask (test 0045's vignette has none: its blend parameters have mask mode 0). Each is test 0045's sidecar with only the vignette entry's params edited:
  - `vignette`: test 0045 with dithering off, its only edit (Dithering): fall-off from 69.4% over 32.08%, brightness −0.968, saturation −0.82, centre (−0.116, 0.13), fixed ratio 1.172 (the `whratio > 1` branch), shape 1.49, unbound.
  - `vignette-autoratio`: the automatic ratio, shape 0.83, fall-off from 41.3% over 77.7%, brightness −0.613, saturation −0.271, centre (0.237, −0.168), unbound.
  - `vignette-narrow`: fixed ratio 0.63 (the `whratio ≤ 1` branch), shape 0.37 (a rounded rectangle), fall-off from 23.7% over 112.3%, brightness −0.57, saturation +0.29 (more saturated towards the edges), centre (0.071, 0.311), unbound.
  - `vignette-star`: shape 3.7 (a four-pointed star, `exp1` below 1), fixed ratio 1.61, fall-off from 31.7% over 83.6%, brightness −0.81, saturation −0.37, centre (−0.29, −0.077), unbound.
  - `vignette-clip`: clipped (`unbound` off), a white vignette, brightness +0.413 and saturation +0.372, fixed ratio 0.87, shape 1.13, fall-off from 52.9% over 46.3%, centre (−0.043, 0.197).
  - `vignette-white`: a white vignette unclipped, brightness +0.37 (added to every channel, past 1 where the input is bright) and saturation −0.43, fixed ratio 1.37, shape 1.71, fall-off from 47.3% over 61.9%, centre (0.153, 0.087). It checks positive brightness without clipping against darktable, not only against the GPU test's CPU transcription.
  - Together: both ratio branches and the automatic ratio, shapes from 0.37 to 3.7, centres off the middle in every direction, both signs of brightness and saturation, each clipped and unclipped.
  - Kinks: no edited `whratio` is 1 and no brightness 0, and no centre lies on a pixel's column or row of either image (the nearest pixel is 0.02 of a pixel from an axis). The fall-off's edges are curves across a continuum of positions, so pixels come arbitrarily close to them (within 1e-7, in the vignette's units, on the test image); what matters is how many sit within a finite-difference step of an edge. So the slots were chosen by a float64 emulation of the check, on the sweep and under small moves of every setting (Gradients).
- **Tolerance:** `Tolerance::DARKTABLE` (mean ΔE2000 ≤ 0.767, max ≤ 2.3) for every case.
- **Invariants:**
  - On the GPU (`crates/golden/tests/vignette.rs`, run in CI, one `GpuContext` shared through `OnceLock`):
    - the kernel follows darktable's `process()` (a CPU transcription, without dithering, within 2e-5) on a gradient image with channels below 0 and above 1 for every case's settings, darktable's defaults and "lomo" preset, ratios from 0.05 to 1.95, the automatic ratio, shapes from 0.2 to 5, the minimum fall-off, no inner radius, a radius beyond the image, the centre at the corners, clipped and not, alpha passing through;
    - brightness 0 and saturation 0 with `unbound` give every pixel back bit for bit, whatever the geometry;
    - output stays finite for 0, ±1e6, 1e-30 and negative input at the extreme settings (shapes 0 and 5, ratios 0, 1e-4 and 2, radii 0 and 200, centre at ±1, strengths at ±1, clipped or not);
    - at shape 0 the vignette is a rectangle with its inner radius where darktable's underflows;
    - every parameter's derivatives, the saturation's included, match finite differences with unequal channel weights, on test 0045's settings and on settings with a ratio below 1 chosen by emulation for their margin (Gradients);
    - a tiled export (after exposure, at its identity, in tiles of 37×33, 64×1000 and 1000×40 over 150×101) equals the whole one bit for bit, and the whole one the kernel's own output;
    - the viewport pipe at scales 0.5 and 0.3 (400×300 into 200×150 and 120×90) gives darktable's `process()` at that scale, with the full-resolution size as `buf_in`, for every case, the white one included.
  - On the CPU (`setup.rs`, `lib.rs`): dithering is `NotPorted`; the ratio is kept within [0.001, 1.999]; values that are not finite are `NotPorted`; the uniform's layout; a gradient slot moves its parameter; the schema's defaults are darktable's, and every parameter reaches the settings; the manifest agrees with the kernel about derivatives.
  - In `ports.rs`: test 0045's stored params read as darktable stores them, but for dithering, which the case turns off; every case has kernel inputs and its gradient slots, each moving the kernel's inputs, and every parameter but the saturation is checked on some case.

## Changes after the reference release

`git log $DT_TAG..upstream/master` over `vignette.c`, `extended.cl` and `tea.h`, upstream/master at 37d514c78c, 2026-10-14:

- **No effect on darktable's CPU output:** c2dd84a409 (the automatic ratio's checkbox becomes a bauhaus toggle, GUI) and 39df8424e6 (an unused `<assert.h>` removed).
- **OpenCL only:** d3e1441978 replaces `pow`, `cos` and `read_imagef` with darktable's `dtcl_pow`, `dtcl_cos` and `readpixel` in the vignette kernel, among OpenCL tiling fixes; the fall-off curve and the seeds are unchanged.
