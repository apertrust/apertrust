# Colour calibration

Module `channelmixerrgb`, version 1. Background only, with no Auto.

It is darktable's color calibration as its page has it, every tab, the CAT tab first (decision 27: "keep both modules the same as in darktable"; [README.md](README.md), Colour calibration). It owns every parameter of its operation, `channelmixerrgb` (working 28.5, `docs/ports/channelmixerrgb.md`). It covers the light the photo is adapted for, a channel mix, a camera calibration's matrix and a black and white conversion. The raw-stage white balance is White balance's ([white-balance.md](white-balance.md)), as in darktable.

Every colour raw runs the operation (decision 27), so the module adds none. It has no first step, and at its defaults it sets nothing and leaves the plan, the cache keys and the frame as they were.

**Status:** built, every tab, in `crates/modules/src/colour_calibration.rs`. It is in `MODULES` after Exposure. The app draws it in the Colour tab from its definition (`docs/gui.md`, Module panels from definitions), after RGB primaries and before Input colour profile, whose module is not built yet (decision 38).

- As shot is darktable's `reload_defaults`, snapped to daylight or Planckian where darktable snaps ([As shot](#as-shot)).
- The adaptation is unavailable while as shot in camera is chosen, through `Module::available` of the values (decision 30).
- Every row below is a test in `crates/modules/tests/colour_calibration.rs`; [Results](#results) gives what they measured.
- Every row is exact or darktable's own match. Two proposed bounds are open (decision 30): C5's on the hue, which fails at low chroma, and C1's on an illuminant reset control by control.
- The [design choices](#design-choices) are settled. Measures and test images are in [README.md](README.md).

## Controls

Each control is darktable's parameter as it is:

- **No transfer function,** darktable's hard range and darktable's default, which is the operation's for the photo. On a colour raw that is CAT16 and As shot ([below](#as-shot)), and the identity mixer, every weight 0, no normalisation, which `Operation::defaults` never changes.
- **darktable's soft range,** the span its track shows, and darktable's step, what `dt_bauhaus_slider_get_step` gives over the soft range: 0.01 for every mixer slider, with darktable's three decimals there ([Sliders against darktable](#sliders-against-darktable)).
- **The content tabs are darktable's notebook pages,** in its order, the CAT tab first and the mixer's with darktable's four rows each. The colour checker's group sits under every tab, as darktable draws its section under the notebook (`Definition::after`).
- **A choice's options are darktable's labels.** Their ids, which edits and scripts write, are adaptation's `linear_bradford`, `cat16`, `non_linear_bradford`, `xyz`, `none`; illuminant's `pipeline`, `a`, `d`, `e`, `f`, `led`, `planckian`, `custom`, `camera`; the F sources' `f1` to `f12`; the LED sources' `b1` to `b5`, `bh1`, `rgb1`, `v1`, `v2`.

| Tab | Control | Parameter | Kind and range | Default |
| --- | --- | --- | --- | --- |
| CAT | `adaptation`: Adaptation, first | `adaptation` | Choice: linear Bradford (ICC v4), CAT16 (CIECAM16), non-linear Bradford, XYZ, none (bypass), all ported; unavailable while `illuminant` is as shot in camera, which forces CAT16, as `gui_changed` makes it insensitive (decision 30); it still takes a value | CAT16 |
| CAT | The CCT readout, the illuminant's colour and the picker, on one row | None: computed from the values (`_update_approx_cct`: "CCT: N K" with daylight, black body or invalid, or "CCT: undefined") | Left out of the definition until a row draws a text and a swatch the module computes, as Exposure's computed EC waits; hidden with adaptation none | — |
| CAT | `illuminant`: Illuminant | `illuminant` | Choice: same as pipeline (D50), A (incandescent), D (daylight), E (equi-energy), F (fluorescent), LED (LED light), Planckian (black body), custom, as shot in camera, darktable's nine (the detection modes, compiled out, are not listed), all ported, bound by darktable's values, as shot in camera's 10; darktable lists the last on a colour raw only (`reload_defaults`), Apertrust on every photo until a choice's options can follow the photo; hidden with adaptation none | As shot: D, Planckian or custom, as `_check_if_close_to_daylight` describes the camera's own light |
| CAT | `illum_fluo`: F source | `illum_fluo` | Choice: F1 (Daylight 6430 K) – medium CRI to F12 (Tuned RGB 3000 K) – low CRI, darktable's twelve labels; shown for F | F3 |
| CAT | `illum_led`: LED source | `illum_led` | Choice: B1 (Blue 2733 K) to B5, BH1, RGB1, V1 (Violet 2724 K), V2, darktable's nine, with darktable's table (V1's entry, which looks wrong against CIE 015:2018, kept under the mirror rule); shown for LED | B5 |
| CAT | `temperature`: Temperature | `temperature` | Number, 1667 to 25000 K, the track over 3000 to 7000, 0 decimals, step 1; shown for D and Planckian | The light's correlated colour temperature |
| CAT | `hue`, `chroma`: Hue, Chroma, coordinates of the view `hue_chroma` of `x` and `y` | — | Hue 0 to 360°, no soft range, 1 decimal, step 1, no fill; chroma 0 to 300 %, the track over 0 to 100, 1 decimal, step 1; shown for custom | As shot's |
| CAT | `x`, `y`: no rows of their own, the view shows them | `x`, `y` | Number: x −0.1 to 1.2 and y 0.0001 to 2, the operation's, widened to hold every hue and chroma (decision 27) | The camera's own light |
| CAT | `gamut`: Gamut compression | `gamut` | Number, 0 to 12, the track over 0 to 4, 2 decimals, step 0.05 | 1 |
| CAT | `clip`: Clip negative RGB from gamut | `clip` | Toggle | On |
| CAT: Area colour mapping, a collapsible group, closed at first | `area_mode`: Area mode, `channel_mixing`: Take channel mixing into account; the Target label with `target_lightness`, `target_hue` and `target_chroma`: Lightness, Hue and Chroma; the input patch with its L, h and c and the target patch come with the picker | None: darktable's settings for its picker, kept in its darktablerc (`darkroom/modules/channelmixerrgb/use_mixing`, `lightness`, `hue`, `chroma`; the area mode reset to correction on every `gui_update`), never in an edit (rule 12); `Row::ToolSetting`s drawn disabled until the picker comes; titled "area colour mapping (active)" in correction mode with the target off 50 % and 0 | Correction, measure; toggle; lightness 0 to 100 %, hue 0 to 360° with no fill, chroma 0 to 128 with no unit, each 1 decimal | Correction, on, 50 %, 0°, 0 |
| R | `red_r`, `red_g`, `red_b`: Input R, Input G, Input B | `red_0`, `red_1`, `red_2` | Number, −2 to 2 (darktable's `COLOR_MIN` to `COLOR_MAX`), step 0.01, three decimals | 1, 0, 0 |
| R | `red_normalise`: Normalise channels | `normalize_R` | Toggle | Off |
| G | `green_r`, `green_g`, `green_b`; `green_normalise` | `green_0` to `green_2`; `normalize_G` | As R | 0, 1, 0; off |
| B | `blue_r`, `blue_g`, `blue_b`; `blue_normalise` | `blue_0` to `blue_2`; `normalize_B` | As R | 0, 0, 1; off |
| Colourfulness | `colourfulness_r`, `colourfulness_g`, `colourfulness_b`; `colourfulness_normalise` | `saturation_0` to `saturation_2`; `normalize_sat` | Number, −2 to 2, its track over −1 to 1, step 0.01, three decimals; toggle | 0; off |
| Brightness | `brightness_r`, `brightness_g`, `brightness_b`; `brightness_normalise` | `lightness_0` to `lightness_2`; `normalize_light` | As Colourfulness | 0; off |
| Grey | `grey_r`, `grey_g`, `grey_b`; `grey_normalise` | `grey_0` to `grey_2`; `normalize_grey` | As Colourfulness, its track over 0 to 1 | 0; off |
| Under every tab: Calibrate with a colour checker, a collapsible group, closed at first | Chart, Optimise for, Patch scale | None: darktable's settings for its colour checker tool, kept in its darktablerc (`darkroom/modules/channelmixerrgb/colorchecker`, `optimization`, `safety`), never in an edit (rule 12). Each is a `Row::ToolSetting`, drawn disabled at darktable's default until the tool comes | darktable's seven charts; none, neutral, saturated, skin and soil, foliage, sky and water colours, average ΔE, maximum ΔE; 0 to 1, three decimals | The first chart, none, 0.5 |

"Input R, G and B" are the adaptation space's channels, CAT16's LMS by default, not working RGB's, as in darktable (`docs/ports/channelmixerrgb.md`, Parameters). A script sends `SetControl { module: "channelmixerrgb", control: "grey_g", value: 1 }`, and `apertrust-cli edit --set channelmixerrgb.grey_g=1` makes a photo grey. `--param channelmixerrgb.grey_1` is refused, since the module owns it.

**Owned with no control:** `version`, darktable's saturation algorithm, at version 3. darktable shows its combobox only for an edit made with an older algorithm (`gui_update` hides it at version 3). No Apertrust edit is one, since no darktable edit is imported (decision 51) and versions 1 and 2 are not ported. Rule 12 leaves such a control out, its parameter at darktable's current value. The module owns it, so no other path sets it.

**The CAT tab** (decision 27). Its rows show as darktable's `_update_illuminants` shows them:

- with adaptation none, every illuminant row, the readout row and the picker hide;
- F source shows for F, LED source for LED, temperature for D and Planckian, hue and chroma for custom;
- none of them shows for same as pipeline, A, E and as shot in camera.

An edit that holds one of its parameters as an operation parameter, set through the engine's unowned-parameter path (decision 28's `--param`, `Command::Set`), loses it with a warning as the edit is read.

- **The stored light** is `x` and `y`, read by custom and as the camera's fallback; `temperature` is read by D and Planckian, the sources by F and LED, and as shot in camera reads the camera's light from the image description at every `prepare`, forcing CAT16.
- **Hue and chroma** are darktable's `dt_xyY_to_Lch` at Y 1 and `dt_Lch_to_xyY` at L 100, CIE Luv relative to D50: `view` and `unview`, in `f64`, inverting each other to rounding. At chroma 0 the hue has no meaning; it holds where it was while the session holds the view, as darktable's slider stays.
- **The widened `x` and `y`.** darktable's chroma reaches 300, past darktable's own bounds on `x` and `y`: x + y passes 0.9999 from chroma 103.6 at hue 79°, y passes 1 from 217.5, x goes below 0 from 272.0. So the operation's `x` runs from −0.1 to 1.2 and `y` from 0.0001 to 2, holding the sliders' reach, x −0.050 to 1.096 and y 0.112 to 1.879. `prepare` still scales a pair whose sum passes 0.9999 back onto it (`docs/ports/channelmixerrgb.md`, Deliberate differences).
- **Follow writes** ([README.md](README.md), rule 13) are darktable's `gui_changed` and `_illum_xy_callback`:
  - Choosing an illuminant, an F or LED source or a temperature writes `x` and `y` to the light it describes (`illuminant_to_xy`), unless the illuminant is custom or as shot in camera. It writes `temperature` to the light's correlated colour temperature (`_check_if_close_to_daylight`), unless it is D, Planckian or as shot in camera.
  - Editing hue or chroma writes `temperature` (`xy_to_CCT`, `CCT_reverse_lookup` below 3000 K).
  - Choosing as shot in camera first writes `x` and `y` from the camera, at the camera's reference where the raw stage leaves the image (`find_temperature_from_raw_coeffs` with no correction), the temperature `_check_if_close_to_daylight` gives it, and the adaptation CAT16.
  - Each is in the same change as the control, one render and one write (the session has no history yet). Each is kept unless it writes the control's default for the photo, which resets it, and none rewrites the control the command set (`Module::follow`, applied by `Edit::follow`).
- **Leaving a light.** A switch from daylight to custom keeps the light, as in darktable, and so does leaving as shot in camera. darktable reads the camera's light again then, which choosing it wrote already: `Module::follow` is given the values after the change, not the illuminant before it ([Design choices](#design-choices)). With the features Apertrust has, the two differ only where a script moved `x`, `y` or the adaptation while as shot in camera was chosen. darktable reaches two more:
  - a preset, a style or a pasted edit that sets as shot in camera over another light, which leaving it then replaces with the camera's; Apertrust has no presets, styles or pasting for the module yet;
  - a white balance changed while as shot in camera is chosen, since darktable's camera light follows `temperature`'s coefficients (`custom_wb`), where Apertrust's is read at the camera's reference. White balance reaches this, and its page records it as a gap (`white-balance.md`, Gaps): `camera_light` assumes the reference coefficients, `custom_wb` 1, and `follow` cannot see `temperature`'s values.

  The render agrees with darktable's either way, since `prepare` reads the camera's light through the coefficients the image description carries (`docs/ports/channelmixerrgb.md`, Setup on the CPU, 7). What differs is the light written into `x`, `y` and `temperature`, which leaving as shot in camera for custom, D or Planckian then keeps.
- **Not mirrored:** darktable's memory of the last daylight and Planckian temperatures of its session (`last_daylight_temperature`, `last_bb_temperature`, reset to the default when the photo opens), which no edit stores. Choosing D or Planckian takes the stored temperature, the last light's (principle 5's first pass).
- **A temperature a follow write stores** is held to the control's range, 1667 to 25000 K, as the operation's defaults hold one. darktable stores the correlated colour temperature whatever it is, which only modes that do not read it reach.

### As shot

As shot, the default illuminant on a colour raw, is darktable's `reload_defaults` (`docs/ports/channelmixerrgb.md`, Defaults). The camera's own light is in `x` and `y`, described as daylight or Planckian at its correlated colour temperature where either lies within 0.005 in darktable's UCS, and as custom otherwise. So an unedited photo's CAT page and render are darktable's, and the controls' defaults are the operation's (C0).

It also decides what a reset of the illuminant gives (C1). Where As shot is custom, the light the illuminant wrote stays, a custom one. Where As shot snaps to daylight or Planckian, a reset of the illuminant returns to it; those read the temperature, not `x` and `y`, so the photo renders as untouched while the temperature is.

**What the panel does not draw yet,** each a gap of the renderer or of a tool still to come:

- the CAT tab's readout row: the CCT readout and the illuminant's colour, which a row drawing a text and a swatch the module computes will bring, and the picker beside them, with its tool;
- as shot in camera listed on a colour raw only, as darktable's `reload_defaults` lists it: a choice's options do not follow the photo yet, so it is listed on every photo, and on one with no camera's light it renders the stored light through CAT16;
- hue's track painted round the hues and chroma's from grey to the hue's colour, the temperature's along the loci, and the area mapping target's tracks, which painted tracks will bring (the module panel's row kinds); plain meanwhile;
- the area colour mapping's input patch with its L, h and c, its target patch and its title's "(active)", which come with the picker; its settings are drawn disabled meanwhile;
- the area colour mapping target's units, its lightness in % and its hue in °: a tool's setting has no unit yet, so screen readers are told the number alone;
- the colour checker's ΔE readout and its Validate, Recompute and Accept buttons, and the chart's frame on the photo, which need the picker and on-photo tools (decision 16); Accept will write this module's mixer and illuminant as one history step;
- darktable's tab tooltips, "output R", "output G", "output B", "output colorfulness", "output brightness" and "output gray": a definition's tab has no words but its title;
- darktable's sliders painted with the colours each input gives (`_update_RGB_colors`): the tracks are plain;
- the colour checker's settings kept in settings, which come with the tool;
- darktable's presets (monochrome films, basic channel mixer, channel swaps), Phase 3.

### Sliders against darktable

Audited against darktable's `src/iop/channelmixerrgb.c`, its params and `gui_init`'s `NOTEBOOK_PAGE` ([README.md](README.md), Sliders against darktable). Every slider shows its parameter as it is, with no factor or format, and three digits (`dt_bauhaus_slider_set_digits(…, 3)`, and `dt_bauhaus_slider_new_with_range`'s for the patch scale). `its_sliders_are_darktable_s`, in `crates/modules/src/colour_calibration.rs`, holds the definition to this table.

| Sliders | darktable: range, soft range | Digits | darktable's step | Apertrust: range, track | Decimals, step | Differs |
| --- | --- | --- | --- | --- | --- | --- |
| R, G and B tabs' Input R, G and B | −2 to 2; none | 3 | 0.01 | −2 to 2; the range | 3; 0.01 | No |
| Colourfulness and Brightness tabs' Input R, G and B | −2 to 2; −1 to 1 | 3 | 0.01 | −2 to 2; −1 to 1 | 3; 0.01 | No |
| Grey tab's Input R, G and B | −2 to 2; 0 to 1 | 3 | 0.01 | −2 to 2; 0 to 1 | 3; 0.01 | No |
| Patch scale, the colour checker's tool setting | 0 to 1; none | 3 | 0.01 | 0 to 1; the range | 3; 0.01 | No |
| CAT: Temperature | 1667 to 25000 K; 3000 to 7000 | 0 | 1 K | 1667 to 25000 K; 3000 to 7000 | 0; 1 | No |
| CAT: Hue | 0 to 360°; none | 1 | 1° | 0 to 360°; the range | 1; 1 | No |
| CAT: Chroma | 0 to 300 % (`set_hard_max`); 0 to 100 | 1 | 1 % | 0 to 300 %; 0 to 100 | 1; 1 | No |
| CAT: Gamut compression | 0 to 12; 0 to 4 | 2 | 0.05 | 0 to 12; 0 to 4 | 2; 0.05 | No |
| CAT: Area colour mapping's lightness, hue and chroma, tool settings | 0 to 100 %, 0 to 360°, 0 to 128; none | 1 | 1 | the same | 1; 1 | No |

The Colourfulness, Brightness and Grey tabs' tracks span darktable's soft ranges; a value typed or scripted beyond one widens the track to show it until the row is reset. The CAT tab's rows are read from `gui_init`, as the table gives them, and `its_sliders_are_darktable_s` holds them too, with no difference.

## Invariants

A first-pass table (decision 22): the rows every table has, and darktable's match. The module adds no operation, so it has no first step, and no control changes a unit, so no row bounds one.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| C0 | None: every control at its default, written or kept | | `bind`; the module's defaults against the operation's for the photo; the plan's instances and cache keys; the frame | `bind` returns nothing; every default is the operation's; instances, keys and frames as with no edit | Exact | Ramp, sweep; the small raw through the export and the viewport; every corpus raw (defaults, instances and keys) |
| C1 | Each control, moved and reset, as the session does both, with their follow writes | A weight on each tab; a weight with its Normalise channels on the G, Colourfulness, Brightness and Grey tabs; every mixer control at once; on the CAT tab the adaptation XYZ and none, same as pipeline, A, D, E, Planckian at 3200 K, F11, LED B1, a custom light, gamut compression 4, and 0 with clipping off; each reset by `ResetControl` and by `ResetModule` | Frame | Changes when moved; as C0 once reset (proposed; an illuminant's move reset control by control keeps its light, Results) | Exact | Sweep (the mixer's), whose synthetic light is D50's; the small raw through the viewport (all) |
| C2 | Order | Two controls, each set first, as the session sets them, with their follow writes: Input G on R with Input B on Colourfulness, Input R on Grey with its Normalise channels, Input R on Brightness with Input B on G, D with gamut compression, the adaptation XYZ with clipping off, F with Input G on R | Frame | The same whichever came first | Exact | Sweep |
| C3 | On a layer | | The definition | Background only: never offered on a layer (Phase 4 adds layers) | Exact | CPU |
| C4 | darktable's match | Each `channelmixerrgb` golden case's parameters, the CAT tab's with the mixer, colourfulness, brightness, grey and normalisations, set as the module's controls | The parameters the edit renders | darktable's own for the case, `version` included, so the case's render against darktable is the module's | Exact; the cases' `Tolerance::DARKTABLE` | CPU; the eleven golden cases |
| C5 | The CAT tab's view | Hue across 0 to 360° and chroma across 0 to 300 %; the defaults on every corpus raw | `view` of `unview`; `unview` of `view`; the builtin grid | Each back where it was; every grid point gives `x` and `y` within their ranges | 1e-4 % in chroma and in the hue's arc, its error in degrees times the chroma, as radians, and a few `f32` units in `x` and `y` (proposed); a bound of 1e-4° on the hue itself, proposed too, fails below a chroma of 12 % | CPU |
| C6 | Follow writes | Every illuminant from every other, an F and an LED source, a temperature, hues and chromas through both of darktable's temperatures, the fit and the reverse lookup; as shot in camera chosen and left; a hue moved and reset | `x`, `y`, `temperature`, `adaptation`; the change | darktable's `gui_changed` and `_illum_xy_callback` restated: the same parameters, in one change (the session has no history yet), the control set never rewritten; a hue moved and reset leaves the edit untouched | Exact | CPU, session |
| C7 | The CAT tab's rows | Each illuminant; adaptation none; as shot in camera | The rows shown; `available` | darktable's `_update_illuminants`: each row shown exactly where darktable shows it; the adaptation unavailable with as shot in camera | Exact | CPU |

C4 is how the darktable match is covered: each control is its parameter, so the eleven `channelmixerrgb` cases (`docs/ports/channelmixerrgb.md`, Results) check the module's render, among them `channelmixerrgb` (test 0085: a normalised mixer, colourfulness and brightness), `-grey` (darktable's HP5+ grey weights, normalised) and `-bypass` (mixing in working RGB). Their CAT tab settings are this module's controls too: the `-camera` case's as shot in camera, `-default`'s Planckian fallback, `-daylight`'s D at 4422 K, the adaptations of `-bradford`, `-linear-bradford` and `-xyz`, `-unclipped`'s clipping and the custom ones.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/colour_calibration.rs`; the synthetic and CPU rows run on CI's software GPUs too, the corpus rows with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| C0 | Holds, exactly. `bind` sets nothing with every control written at its default for the photo or kept there, and every default, the CAT tab's As shot among them, is the operation's, bit for bit, on an image that is not a raw, colour raws with a white balance under three matrices (the sample's, whose light stays custom, the Canon EOS 40D's of darktable's test 0174, snapped to daylight at 3927.06 K as darktable stored it, and another), with none and with no matrix, and a monochrome raw. The operation's defaults are darktable's own on the 29 samples the exit test compares (`docs/ports/channelmixerrgb.md`, Results, The defaults). Frames byte-identical to no edit on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that a moved control passes through first. On all 31 corpus raws the pipeline takes, the defaults are the operation's and the instances and cache keys those of no edit (the Leica M Monochrom is among the nine the pipeline does not take yet, so a monochrome raw's defaults are checked on its image context alone) |
| C1 | Holds, but for its proposed bound on the illuminant's moves reset control by control, which fails on the small raw and stays in an `#[ignore]`d test, open (decision 30; below) |
| C2 | Holds, exactly, for the six pairs |
| C3 | Holds: the definition works on the background only, with no Auto |
| C4 | Holds, exactly, on all eleven cases, the CAT tab's parameters with the mixer's: the edit renders darktable's parameters for each. The cases themselves pass `Tolerance::DARKTABLE` against darktable (`apertrust-golden run`, with the widened `x` and `y`): ΔE2000 means of 0.0000 to 0.0001, maxima of 0.0002 to 0.0005, no pixel over 2.3, as the port measured; the gradient check passes on them |
| C5 | The view holds, within its bound on the hue's arc; the proposed bound on the hue itself fails below a chroma of 12 % and stays in an `#[ignore]`d test, open (decision 30; below) |
| C6 | Holds, exactly: every case is darktable's `gui_changed` and `_illum_xy_callback` restated, but for decision 27's two differences, its session memory and an unclamped temperature (below) |
| C7 | Holds, exactly: for each of the five adaptations and nine illuminants the CAT tab shows what `_update_illuminants` shows, and every control is available but the adaptation with as shot in camera, as `gui_changed`'s `gtk_widget_set_sensitive` makes it. Along changes as the session makes them, choosing as shot in camera makes the adaptation unavailable, a value then set is still taken, and leaving it makes it available again; the session reports the same after each change (`a_control_s_follow_writes_are_one_change_with_it`) |

C1 in detail:

- **What holds.** Made as the session makes them, with their follow writes, each of the ten mixer moves changes the sweep's frame and the small raw's, and each of the twelve CAT moves the small raw's. Each renders as no edit once the module is reset. Reset control by control, each does too, the edit then untouched, but for the seven moves of the illuminant with a temperature or a source.
- **What fails.** A reset of the illuminant keeps the light it wrote into `x` and `y`, kept, so the photo renders that light as a custom one, not As shot. That is darktable's `gui_changed` where the illuminant's default is custom, as the small raw's As shot is, its light too far from both loci to snap.
- **On a photo whose As shot snaps,** the Canon EOS 40D of darktable's test 0174 (daylight at 3927.06 K, rendered as the Planckian), measured on the CPU by the light `prepare` takes. Each of the seven moves, reset control by control with the temperature reset too, renders As shot's light exactly, since daylight and Planckian read the temperature, not `x` and `y`. Reset without the temperature, as the bound has it, only daylight and Planckian at 3200 K do. The five whose follow writes set the temperature to their light's (same as pipeline, A, E, F11, LED B1) keep it, which daylight then reads (`c1_on_a_photo_whose_as_shot_snaps_a_light_reset_with_its_temperature_returns_as_shot`).

C5 in detail:

- Over hue by 0.5° and chroma by 0.5 % (433,321 points), chroma comes back within 6.1e-5 % and the hue's arc, the hue's error times the chroma, within 4.8e-5 %.
- The hue comes back within 9.2e-5° from a chroma of 12 %, but 1.1e-3° at 1 % and 2.2e-3° at 0.5 %, where the rounding of `x` and `y` to `f32` turns a short radius. So the proposed 1e-4° fails below 12 %; CI checks the arc, the chroma and the hue from 12 %.
- `x` and `y` come back within 1.2 `f32` units. Every coordinate of the builtin grid gives `x` and `y` in their ranges, chroma 300 reaching x −0.050 to 1.096 and y 0.112 to 1.879.
- On all 31 corpus raws the defaults lie in the coordinates' ranges, from a chroma of 4.4 % (Fujifilm X100VI) to 91.5 % (Phase One IQ3 100MP), and come back within 4 `f32` units.

C6 in detail: all 81 changes of the illuminant from one to another after the defaults; F7 on F, V1 on LED, 6500 K on daylight and 2700 K on Planckian; a hue and two chromas on a custom light; and as shot in camera chosen after XYZ and left. `x`, `y`, the temperature and the adaptation are darktable's restated in each.

- At hue 40° and chroma 80 %, `xy_to_CCT` gives 2758 K and the reverse lookup takes over.
- At hue 0° and chroma 150 %, it gives 1249 K, below 1667 K: `_illum_xy_callback` looks up 2794 K, while `_check_if_close_to_daylight`, choosing custom from daylight with that light, keeps 1249 K, held to 1667 K.
- A temperature set on a custom light is never rewritten, and every write is kept but one of the control's default, which resets it: a hue moved and reset leaves the edit untouched.
- Through the session (`a_control_s_follow_writes_are_one_change_with_it`), choosing daylight writes its light into `x` and `y`, renders once and saves once, and a module's wrong follow write (`testing::Following`) refuses the whole change.

## Design choices

- **The saturation algorithm:** not shown, `version` owned at 3. darktable hides the combobox at version 3 (`gui_update`), and rule 12 leaves out a control darktable shows only for an older edit.
- **The colour checker's settings:** drawn disabled at darktable's defaults by a row of the definition format, `Row::ToolSetting`: no control, nothing an edit stores, words of its own (`module-<module>-setting-<setting>-…`), the kind giving the widget. The CAT tab's Area colour mapping and Exposure's Area exposure mapping use it too.
- **Control ids:** `<tab>_r`, `_g`, `_b` and `<tab>_normalise`, by the tab's English name (`colourfulness_r` for `saturation_0`), so a script reads as the panel does. darktable's `normalize_R` is no snake_case id.
- **Row ids:** C, for Colour calibration.
- **The CAT tab's ids:** the controls take darktable's parameter names, `illum_fluo` and `illum_led` among them. The options take short ids of darktable's labels (`pipeline`, `planckian`, `camera`, `non_linear_bradford`, `none`), the illuminant bound by darktable's value. The tool settings are `area_mode`, `channel_mixing` and `target_lightness`, `target_hue`, `target_chroma`, apart from the view's `hue` and `chroma`.
- **Leaving as shot in camera:** the camera's light is not read again. `Module::follow` is given the values after the change (decision 27), and choosing as shot in camera already wrote the camera's light and CAT16, so leaving it gives darktable's light in every state the features Apertrust has reach ([Follow writes](#controls), above). Mirroring darktable where a script moved `x`, `y` or the adaptation would need `follow` to be given the control's previous value, as darktable's `gui_changed(self, w, previous)` is: an interface change.
- **Hue or chroma's temperature:** written whenever `x` or `y` moves, as `_illum_xy_callback` writes it whenever the view is edited, whatever the illuminant. darktable's GUI shows the view for custom only, so a script moving `x` under daylight moves the temperature daylight reads.
- **A temperature out of the control's range:** held to 1667 to 25000 K, as the operation's defaults hold one, so a follow write is always one the control takes.
- **Follow writes refused:** the module's failure, `EditError::FollowFailed`: the whole change is refused and the edit stays as it was, as with a preset's option its module gets wrong.
- **A follow write of a control's default:** resets the control, value and mark, as a reset does, so a change and its reset leave the edit untouched (a hue moved and reset).
- **Where the grey runs:** at darktable's position, in the same instance as the CAT tab's adaptation, as built. With layers (Phase 4) the choice is between a second, background-only instance for the grey after every layer's, an exception to decision 34 and the likelier choice, since darktable's film presets assume it, and measuring the masks before the grey (`docs/ports/channelmixerrgb.md`, Pipeline). It decides how the presets apply.
- **The follow writes' test module:** `testing::Following`, Sample with a grain that writes its size, or a wrong write, for the engine's tests. Sample itself is unchanged, so the tests that drive it keep their meaning.

## Phases

- **Built (Phase 2):** the six tabs, and the CAT tab with follow writes and the widened `x` and `y`, with C0 to C7 (decision 27); C7's availability through `Module::available` of the values (decision 30). Neither the CLI's `edit` nor the app's pending display applies follow writes yet: the CLI reads no image context, so `--set channelmixerrgb.illuminant=d` stores the illuminant alone, and the app shows a follow write once the session reports it.
- **With the picker and on-photo tools:** the colour checker: its frame on the photo, the ΔE readout, Validate, Recompute and Accept, its settings kept in settings, and rows for what Accept writes. darktable closes the section on every `gui_update`, and opening it starts profiling (`_start_profiling_callback`: the module takes focus, turns on and frames the chart on the preview). So the group's open state there is the tool's, not only the panel's. Open for the tool: whether opening the group starts it, as in darktable, or a button does, and whether the group then stays closed after every change as darktable's does.
- **Phase 3:** darktable's presets, the grey where it runs as built ([Design choices](#design-choices), Where the grey runs).
