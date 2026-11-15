# Exposure

Tool `exposure`, version 3. It works on layers and has Auto. It drives one operation, `exposure`, which an unedited photo runs, and owns every parameter of it.

Its header's tooltip says what it is for (`tool-exposure-description`, [gui.md](../gui.md#the-headers-tooltip)): "Brightens or darkens the photo as if you had exposed it differently in the camera, keeping its colours. Usually the first tool to reach for."

It is darktable's exposure module in the first pass (decision 2: rewrite first, improve later). It has darktable's page, every control `gui_init` shows, in its order under its labels, on one untitled tab (decision 29): Mode, Compensate camera exposure, Highlight preservation mode, Exposure, Percentile and Target level in automatic mode, Black level correction, and the Area exposure mapping group, with darktable's computed EC readout under Target level ([Controls](#controls); [README.md](README.md)). Every control is ported.

Exposure is darktable's slider, `exposure.exposure` itself, its default the photo's: +0.7 EV on a colour raw, 0 otherwise (decision 30). Black level correction is darktable's offset over the photo's black (decision 22). Contrast and Pivot, colour balance rgb's `contrast` and `grey_fulcrum`, and Brightness, `midtones_Y`, belong to the Colour balance RGB tool ([colour-balance-rgb.md](colour-balance-rgb.md); decision 2), whose table holds their rows.

**Edits version 1 saved.** Version 1's Exposure solved for exact stops through the photo's black, so its value means another exposure on every photo. Version 2's migration leaves it out with a warning, its mark with it, and Exposure renders at the photo's default; no release wrote such an edit. Black level correction and Compensate camera exposure keep their values. Contrast and Pivot, which an older version 1 had, are left out with a warning each, the photo editable.

**Edits version 2 saved.** Highlight preservation mode was not ported before version 3 and took only off, so an edit of version 2 or 1 reads it off, as it rendered; version 3's default is on, as darktable's ([port spec](../ports/exposure.md)). An edit with no Exposure entry has nothing to migrate and takes the new default, on, as an unedited photo does.

**Status:** built, in `crates/tools/src/exposure.rs`. The app draws it from its definition (`docs/gui.md`, Tool panels from definitions). Its tab is Edit's Adjust, after Colour calibration and before the tone mapper (decision 38).

- Rows E0, E3, E4, E6 to E9, E14, E30 to E37 and E50 are tests in `crates/tools/tests/exposure.rs`, with the migration's; [Results](#results) gives what they measured.
- Automatic mode is built, darktable's deflicker: the raw's histogram counted on the GPU once per photo while it is on, and the exposure it computes shown as a computed value ([Automatic mode](#automatic-mode); [port spec](../ports/exposure.md#automatic-mode-deflicker); decision 87).
- Its picker is built ([The picker](#the-picker); [pickers.md](../pickers.md), decision 79): its quad on Exposure, its pick in both area modes, the area exposure mapping's settings kept in settings, and the patches with the L readout.
- A tolerance or a choice marked "proposed" is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

In the first pass Exposure is darktable's page, every control on it in `gui_init`'s order, on one untitled tab with no toggle (decision 29). The Row column gives each control's label and when it shows: darktable shows Compensate camera exposure, Highlight preservation mode and Exposure in manual mode and Percentile, Target level and the computed EC readout in automatic mode (`Row::When`).

| Control | Row | Kind and range | Default | Changes | Keeps | Binding |
| --- | --- | --- | --- | --- | --- | --- |
| `mode` | Mode, always | Choice: manual; automatic, darktable's deflicker | Manual | How the exposure is set: by hand, or from each photo's raw histogram | Black level correction, in both | `exposure.mode` = the option, set only where set. Drawn disabled on an image that is not a raw, as darktable makes it insensitive on any but a raw of one 16-bit channel, the only raws the engine develops; there an edit holding automatic renders as manual mode, as darktable's `commit_params` leaves it |
| `camera_bias` | Compensate camera exposure, in manual mode | Toggle | The photo's: on for a colour raw | Whether the camera's exposure compensation is undone | Everything, on a photo with no bias | `exposure.compensate_exposure_bias` = the value, set only when it differs from the photo's |
| `compensate_hilite_pres` | Highlight preservation mode, in manual mode; drawn disabled on a photo whose camera held no exposure back, where darktable hides it | Toggle | On, as darktable's first instance | Whether the exposure the camera held back in a highlight-preserving mode is given back: Canon's Auto Lighting Optimizer, Fujifilm's DR200 and DR400, Nikon's HLG and Active D-Lighting, Olympus's gradation, as darktable estimates it from the maker notes ([port spec](../ports/exposure.md#the-held-back-exposure-_check_highlight_preservation)) | Everything, on a photo with none held back | `exposure.compensate_hilite_pres` = the value, set only where set; the operation adds the held-back exposure, within −1 to +4 EV, after taking off the camera's bias, as darktable's `commit_params` does |
| `exposure` | Exposure, in manual mode, with darktable's picker on its quad (E50) | Number, −18 to +18 EV, darktable's range, step 0.05, its track over darktable's soft range, −3 to +4 EV (decision 30) | The photo's: +0.7 EV on a colour raw, 0 on a monochrome raw or an image that is not a raw, as darktable's `reload_defaults` sets it | The brightness of every pixel | Contrast and colour: the ratio between any two pixels or channels | `exposure.exposure` = the value, darktable's slider's; the operation subtracts the camera's bias while Compensate camera exposure is on, as darktable's `commit_params` does |
| `deflicker_percentile` | Percentile, in automatic mode | Number, 0 to 100 %, step 1 | 50 %, the median | Where automatic mode meters the raw's histogram: the raw value that share of the declared image's photosites reaches | Contrast and colour | `exposure.deflicker_percentile` = the value, set only where set |
| `deflicker_target_level` | Target level, in automatic mode | Number, −18 to +18 EV, step 0.1 | −4 EV | Where automatic mode puts the metered value, in EV below the white point | Contrast and colour | `exposure.deflicker_target_level` = the value, set only where set; the operation applies `target − ev`, ev the metered value's `log2(max(v − b, 1)) − log2(w − b)` on rawprepare's black b and white w, in place of Exposure and both compensations |
| `black_level` | Black level correction, always | Number, −1 to +1, darktable's range, step 0.001, its track over darktable's soft range, −0.1 to +0.1 (decision 30) | 0: the photo's black | Where black sits, as darktable's black level correction | The white point | `exposure.black` = the photo's default (−2⁻¹² for a colour raw) plus the value, within darktable's ±1. In manual mode the tool puts a black that reaches the white the photo renders, w = 2^−(e − β), 0.01 below it, darktable's margin, so the kernel never divides by zero (decision 30). In automatic mode the tool binds it within ±1 alone, and the operation's `prepare` puts a black that reaches the white the photo renders, 2^−e at the computed exposure or at manual mode's without a histogram, 0.01 below it, the tool's own rule (decision 87) |

**Area exposure mapping**, last, is darktable's collapsible section, closed at first. It holds darktable's settings for its picker, which no edit stores ([README.md](README.md), rule 12), kept in settings as `tools.exposure.area_mode` and `tools.exposure.lightness` (`Row::Setting`): Area mode, correction or measure, correction by default; the input's and the target's patches side by side under their headings, Input and Target, the input's L readout under its patch (`Row::Patches`); then Lightness, the target, 0 to 100 %, 50 % by default, as darktable's darktablerc has it.

- **The input patch** is the area the last pick read on the photo, made grey of its lightness, and its readout darktable's "L: 45.2%", "L: N/A" before a pick; both stay as the picker turns off, and go, back to "L: N/A", where darktable's `gui_update` zeroes them (`exposure.c:707-710`): undo, redo, a move through history, the edit's reset or Exposure's, another photo.
- **The target patch** is the grey of Lightness, as darktable paints it, and Lightness shows its percent, its track darktable's ramp of the greys of each lightness (`_paint_hue`, `exposure.c:1123-1151`).
- darktable lays the target's patch and Lightness in a column beside the input's; the side panel's width puts Lightness on a row of its own under both.
**Computed EC**, under Target level in automatic mode, is darktable's readout of the exposure automatic mode applies, `%.2f EV` (`exposure.c:1236-1245`): a computed value (`Row::Computed`), drawn as the design system's readout row (`ReadoutRow`), its label "Computed EC" at the start and the value at the end, "+0.51 EV", the tooltip darktable's "What exposure correction has actually been used." The session reports it with the values, from the exposure operation's `computed` on the edit as it renders (`ToolState::computed`), so it shows as soon as the photo is live and its histogram counted, where darktable's stays blank until its preview pipe has run. Where automatic mode has no histogram, on an image that is not a raw or after a count that failed, it is blank, and the photo renders as manual mode.

**What is not ported.** Nothing: Mode, Percentile and Target level are ported with automatic mode, and every value of their ranges is taken (E32).

**Owners.** Exposure owns every `exposure` parameter, darktable's `mode`, `black`, `exposure`, `deflicker_percentile`, `deflicker_target_level`, `compensate_exposure_bias` and `compensate_hilite_pres`. Contrast, Pivot and Brightness are the Colour balance RGB tool's `contrast`, `grey_fulcrum` and `midtones_y`, darktable's parameters, and Colour balance RGB owns every `colorbalancergb` parameter (decisions 28 and 32).

The control `exposure` is the operation parameter of the same name, which the tool owns. So `apertrust-cli edit --set exposure.exposure=1.7` sets +1.7 EV, 1 EV over a colour raw's default, while `--param exposure.exposure` is refused. A script sends `SetControl { tool: "exposure", control: "exposure", value: 1.0, done: true }`, which is the same command a drag ends with.

### Sliders against darktable

Audited against darktable's `src/iop/exposure.c`, its params and `gui_init` ([README.md](README.md), Sliders against darktable). darktable's columns are its own, in the units its slider shows: no slider here has a factor or an offset (Percentile's "%" format gives none, its range being above ±10). `its_sliders_are_darktable_s`, in `crates/tools/src/exposure.rs`, holds the definition to this table and finds no difference (decision 30).

| Slider | darktable: range, soft range | Digits | darktable's step | Apertrust: range, track | Decimals, step | Differs |
| --- | --- | --- | --- | --- | --- | --- |
| Exposure (`exposure`) | −18 to +18 EV; −3 to +4 EV | 3 | 0.05 EV | −18 to +18 EV; −3 to +4 EV | 3; 0.05 EV | No. The value shown is darktable's, +0.700 EV on a colour raw as first shown. A value typed or scripted beyond the soft range widens the track, as darktable's does |
| Percentile (`deflicker_percentile`) | 0 to 100 %; none | 2 | 1 % | 0 to 100 %; the range | 2; 1 % | No |
| Target level (`deflicker_target_level`) | −18 to +18 EV; none | 2 | 0.1 EV | −18 to +18 EV; the range | 2; 0.1 EV | No |
| Black level correction (`black_level`) | −1 to +1, the black itself; −0.1 to +0.1 | 4 | 0.001 | −1 to +1, an offset over the photo's own black (decision 22); −0.1 to +0.1 | 4; 0.001 | No. darktable's slider shows the black, −0.0002 on a colour raw, ours the offset over it, 0 there. The black is held within darktable's ±1, so on a colour raw the control's bottom 2⁻¹² changes nothing, and one that reaches the white the photo renders goes 0.01 below it (decision 30) |
| Lightness (`lightness`), Area exposure mapping's setting | 0 to 100 %; none | 1 | 1 % | 0 to 100 %; the range | 1; 1 | No |

Exposure steps by darktable's 0.05 EV, Shift 0.5 EV and Alt 0.005 EV, darktable's coarse and fine steps, growing as a widened track's span does, as darktable's zoom step makes it. A drag and a typed value round to the three decimals shown, as darktable's do. Exposure's range and Black level correction's are darktable's (decision 30); widening a range raises no tool version (decision 43).

### Tooltips

These are the tooltips (`tool-exposure-control-<control>-promise`), each turned into the rows named. Exposure, Highlight preservation mode, Percentile and Target level have darktable's tooltips, and so does the computed EC readout; Mode, which has none in darktable, has words of the same kind.

| Control | Tooltip | Rows |
| --- | --- | --- |
| Exposure | Adjusts the exposure correction. | E4, E6 to E9 |
| Black level correction | Sets which level is black. Keeps white. 0 = untouched. | E30 |
| Compensate camera exposure | Undoes the exposure compensation set on the camera. | E31 |
| Highlight preservation mode | darktable's | E33 |
| Mode, Percentile, Target level | darktable's | E32, E34 to E37 |

### Transfer functions

- **Exposure** has none: `exposure.exposure` is the control's value, darktable's slider's. The operation scales `(in − b)` by `1 / (2^−(e − β) − b)`, with e the value, b the black and β what it takes off: the camera's bias while Compensate camera exposure is on, `clamp(bias, −5, +5)` as darktable's `_get_exposure_bias` trusts it, less the held-back exposure while Highlight preservation mode is on, `clamp(h, −1, +4)` where it is above 0 as `_get_highlight_bias` trusts it (`taken_off`). The image context carries both unclamped and `apertrust-tools` cannot call the exposure crate, so `camera_bias` and `highlight_bias` repeat the bounds, and tests hold them to the operation's. So a move of v EV from the photo's default scales every pixel by (w₀ − b) / (w − b), w₀ and w the whites the default and the value render: 2^v where b is 0, slightly less over a colour raw's black of −2⁻¹² (+3.991 EV for +4 EV with no bias), as in darktable. A negative Black level caps the factor at 1 / −b, as darktable's black does. The default is `default_exposure`, darktable's `reload_defaults` restated, which a test holds to the operation's own.
- **`black_level(v)`** (`BlackLevel`): b₀ + v, held within darktable's ±1, with derivative 1 inside; a black at or above the white the photo renders, w = 2^−(e − β) (`rendered_white`), is put at w − 0.01, as darktable's `gui_changed` puts it, with derivative 0. darktable compares with the white of its slider's exposure; this compares with the white the kernel divides by, the camera's bias included. On a colour raw the control's bottom 2⁻¹² holds the black at −1. At the photo's default exposure the white is 0.62 with no bias, 0.39 at −0.67 EV of bias and 0.019 at −5 EV. The photo's own black, 0 or below, never reaches a white, so the black is bound only while Black level is set. The margin has a cost: the kernel computes 2^−e − b, and there the GPU's `exp2` error is amplified up to about 100×, estimated at 2e-4 relative at worst; E30 measures it.

## Invariants

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| E0 | None: every control at its default | | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | Every image |
| E3 | Each control, moved and reset | Exposure +1, Black level correction +0.01, Compensate camera exposure off, Highlight preservation mode off, and the four together; then each reset by double-click, Delete and the header's reset | `bind`; frame | As E0 | Exact | Ramp and a synthetic raw, each holding 0.5 EV back; 90D |
| E4 | Exposure | The photo's default −4, −1, −0.01, +0.01, +1 and +4 EV; −18 and +18, the range's ends | Tool output: each pixel's RGB over E0's | One factor for every pixel and channel, so contrast and colour are kept | Spread ≤ 1e-5 relative, on values over 1e-6 (proposed) | Ramp, sweep; 90D, Z 6, X-T5 |
| E6 | Exposure, across a series | The photo's default +1 EV on photos with biases 0 and −0.67 EV | E4's factor at the scene, in EV | The same on both, so a pasted Exposure means the same on every photo | 0.01 EV (proposed) | Ramp in both contexts; 90D and Z 6 |
| E7 | Exposure | −18, the default ±4 and +18 EV | Frame, on the extended ramp | Finite; a brighter grey never renders darker | Exact order, no NaN or infinity | Extended ramp; 90D |
| E8 | Exposure | E4's values | Tool output: E4's factor | darktable's, (w₀ − b₀) / (w − b₀), at biases 0, −0.67 and −5 EV | 1e-5 relative (proposed) | Ramp with biases 0, −0.67 and −5 EV; 90D, Z 6, X-T5 |
| E9 | Exposure and Black level correction | The golden `exposure` case's values, as darktable's sliders show them | The bound parameters; the kernel's inputs | darktable's parameters, and the inputs the golden harness checks against darktable's render | Exact, bit for bit | None: an image that is not a raw, and a colour raw with its bias not compensated |
| E14 | Exposure with another control | Exposure +1, then Black level correction or Compensate camera exposure; and the other way round | Frame | The same whichever was set first | Exact | Ramp; 90D |
| E30 | Black level correction | −1, −0.1, −0.01, +0.01, +0.1, +1: the range's ends and the track's | Tool output, on the ramp | The level set as black renders 0; the grey Exposure renders at 1.0 stays at 1.0; greys between stretch linearly. Where a value puts black at or above the white the photo renders, black goes 0.01 below it (decision 30): at +1 on photos with a bias up to +0.7 EV, at +0.1 on photos with a bias below −2.6 EV; at −1 on a colour raw it stops at −1, darktable's lowest. The margin amplifies the GPU's `exp2` error, which this row measures there | 1e-5 relative (proposed) | Ramp; 90D |
| E31 | Compensate camera exposure | Off, on a raw with bias −0.67 EV; off, on a raw with none | Tool output: E4's factor | Darkens by the bias, 0.67 EV; with no bias, changes nothing | 0.02 EV (proposed); exact | Ramp in both contexts; 90D, Z 6 |
| E32 | Mode, Percentile and Target level, over their ranges | Percentile 0 and 100; Target level −18 and +18; Mode automatic; each beside Exposure +1, as a command and pasted into a saved edit | The edit | Each taken, Exposure +1 kept, with no warning | Exact | None: the edit |
| E34 | Automatic mode across a series | Automatic at 50 % and −4 EV, Black level correction putting the black at 0, on one scene's raw exposed 2 EV under to 1 EV over | The exposure it computes; tool output | Exposures exactly the brackets apart; one image for the series | 1e-6 EV; 1e-3 relative spread (proposed) | A synthetic raw at 2⁻², 2⁻¹, 1 and 2 times, none clipping |
| E35 | Automatic mode | Percentiles 10, 50 and 90 and target levels −4 and −2 EV | Tool output | The frame manual mode renders at the exposure it computes, with both compensations off, as darktable replaces them | Exact | The synthetic raw holding 0.5 EV back with a bias of −0.67 EV, through the export and the viewport |
| E36 | Automatic mode on an image that is not a raw | Automatic at 90 % and −2 EV, beside Exposure +1 | Frame | Manual mode's, as darktable's `commit_params` leaves it | Exact | Ramp with no camera |
| E37 | The raw's histogram | Exposure +1 in manual mode; then automatic, then its settings moved | Whether the histogram is counted | None in manual mode; counted once in automatic mode, over the declared image, and kept | Exact | The synthetic raw |
| E33 | Highlight preservation mode | Off, on raws whose camera held 0.5 EV and 2 EV back; off, on a raw that held none back | Tool output: E4's factor | Darkens by what was held back; with none held back, changes nothing | 0.02 EV (proposed); exact | Ramp with a bias of −0.67 EV, holding 0.5, 2 and 0 EV back |
| E50 | The picker | A pick on a flat patch, correction, at target lightnesses 30, 50 and 75 % | Exposure's output: the patch's luminance, Y through the input profile the pick reads it through | Becomes the target lightness's grey, 18.4 % at darktable's default 50 %; only Exposure changes | 0.01 EV (proposed) | Five flat patches, four greys from 0.05 to 1 and one of colour, on colour raws with biases 0 and −0.67 EV |
| E51 | Auto, later | Auto, with Exposure kept and not kept | Values, marks and history | Moves only Exposure, and only if it is not kept; marks it Auto; one undo reverts it | Exact | Ramp |
| E60 | Each control on a layer, Phase 4 | E4's and E30's values on a full-mask layer at 100 % | Scene, inside the mask | Changes the image as the same values do on the background | As the row it repeats | Ramp |

E51 waits for Auto's statistic, read back as the pickers' readings are (decision 79).

## Results

Measured on the Apple M4 (Metal) by `crates/tools/tests/exposure.rs`; the synthetic rows run on CI's software GPUs too. The corpus rows run with `APERTRUST_RAW_CORPUS`, the chart's with `APERTRUST_TEST_IMAGES`, both `#[ignore]`d. Where a row fails, its proposed bound stays in an ignored test, open, and CI checks the parts of it that hold; the bound itself is unchanged.

**Where the rows read.** E4, E8, E30 and E31 measure Exposure's own output, the image leaving the exposure operation (the tool output, `docs/tools/README.md`). These rows are about Exposure's maths, darktable's exact multiply. Colour calibration, darktable's default, runs between exposure and AgX on every colour raw (decision 27) and does not keep them at the scene: it raises values below 2⁻¹⁶, clips negative RGB to 0 and compresses colours beyond the gamut, none of which scales with the image.

At the scene, with the synthetic images balanced as below:

- E4 holds on the ramp (a spread of 2.6e-6) and fails on the sweep (1.1e2, its colours beyond the gamut) and on the 90D, the Z 6 and the X-T5, where colour calibration does not scale with the image.
- E30 fails: relative errors up to 1.0, where Black level puts a grey below black and colour calibration clips it to 0, and 3.9e-4 between.
- E31 holds, a spread of 2.3e-6.

At Exposure's output all three hold, as below.

A synthetic image is already white balanced, so the tests' colour calibration adapts it from the pipeline's own illuminant, D50, not the camera's (README, Test images). The camera's would balance it twice, turning the ramp's greys magenta and, at the frame, failing E7 at −4 EV, green clipping to 0 in the shadows.

| Row | Result |
| --- | --- |
| E0 | Holds, exactly. `bind` sets nothing with every control written at its default, or kept there, Mode, Highlight preservation mode, Percentile and Target level included. Frames byte-identical to no edit on the ramp and the sweep at biases 0 and −0.67 EV, and on a small synthetic Bayer raw through the export and the viewport pipes, each on one pipeline that colour balance rgb joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| E3 | Holds, exactly. Exposure +1, Black level +0.01, Compensate camera exposure off, Highlight preservation mode off and the four together, each reset control by control (`ResetControl`) and by the tool's reset (`ResetTool`), on one pipeline that colour balance rgb joins and leaves between the moves, on the ramp and the synthetic raw's viewport, each holding 0.5 EV back; on the 90D, which holds 0.5 EV back, through the viewport and the export. |
| E4 | Holds, at Exposure's output: spreads of at most 2.0e-7 on the ramp and 2.3e-7 on the sweep, the range's ends included; at most 2.4e-7 on the 90D, the Z 6 and the X-T5, with colour calibration on every colour raw |
| E6 | Holds: the default +1 EV gives factors 3.4e-4 EV apart at biases 0 and −0.67 EV, darktable's black weighing the two photos' whites differently |
| E7 | Holds at −18, the default ±4 and +18 EV: every frame finite, no grey darker than a darker one |
| E8 | Holds: within 9.5e-8, 1.2e-7 and 2.4e-7 relative of darktable's factor at biases 0, −0.67 and −5 EV on the ramp; within 3.3e-7 on the 90D, the Z 6 and the X-T5 at every value, −18 and +18 EV included |
| E9 | Holds, exactly: the case's +1 EV and black 0 bind darktable's parameters and the kernel's inputs bit for bit, on an image that is not a raw and on a colour raw |
| E14 | Holds, exactly: one pipeline sets the first control and then both, another the second and then both, for Exposure with Black level and with Compensate camera exposure, on the ramp and the 90D (viewport and export) |
| E30 | Holds, at Exposure's output: within 5.1e-7 relative at −1, −0.1, −0.01, +0.01, +0.1 and +1, at biases 0, −0.67 and −5 EV, the black held at −1 included; where the black goes 0.01 below the white, 1.3e-7, 1.3e-7 and 4.1e-7 at biases 0, −0.67 and −5 EV. lavapipe and WARP wait for CI |
| E31 | Holds, at Exposure's output: off darkens by 0.6697 EV on a −0.67 EV raw, one factor for every pixel within a spread of 1.8e-7; with no bias, exactly nothing changes |
| E32 | Holds, exactly: each command taken and each pasted value kept, with no warning |
| E34 | Holds: the computed exposures +0.9214, −0.0786, −1.0786 and −2.0786 EV, the brackets apart within one f32 step; at Exposure's output the series within spreads of 1.6e-4, 5.3e-5 and 2.6e-5 relative of the scene at 1, at 2 EV under, 1 under and 1 over, demosaic not scale-invariant to the last bit, median factors 1 within 1e-7 |
| E35 | Holds, exactly, at all six settings, in the export and the viewport |
| E36 | Holds, exactly |
| E37 | Holds: nothing counted in manual mode; 84 × 62 photosites counted once automatic is set, the same counts kept as its settings move |
| E33 | Holds, at Exposure's output: off darkens by 0.4996 EV where 0.5 EV was held back and by 1.9973 EV where 2 EV was, one factor for every pixel within a spread of 1.9e-7; with none held back, exactly nothing changes |
| E50 | Holds, at Exposure's output: each pick writes Exposure alone, and every patch lands within 0.0066 EV of its target, at biases 0 and −0.67 EV and targets 30, 50 and 75 %; the furthest is the darkest, 0.05, which the photo's own black of −2⁻¹², left aside by darktable's rule, moves most |

## In the panel

- **Compensate camera exposure is drawn disabled** on a photo that records no bias, as an action that cannot run is (`ui-conventions`), with the tooltip naming the camera's bias as 0. Built through `Tool::available`; the tooltip still says what the control does, since a disabled control's tooltip cannot say why yet (`docs/gui.md`, the renderer's gaps). darktable's labels add the bias, "compensate camera exposure (+0.7 EV)" and "highlight preservation mode (0.5 EV)"; a label with a value of the photo's waits for the renderer.
- **Mode** is a list, manual and automatic. The whole row is disabled on an image that is not a raw, as darktable makes it insensitive; every raw the engine develops is one of 16 bits in one channel, darktable's test.
- **Highlight preservation mode is drawn disabled** on a photo whose camera held no exposure back, as a control that cannot act is (decision 30), on as darktable's default is. darktable hides it there; hiding a control per photo is not decided ([port spec](../ports/exposure.md#open-questions)).
- **Automatic mode** shows Percentile, Target level and the computed EC readout in place of Compensate camera exposure, Highlight preservation mode and Exposure, whose picker it hides with it, as darktable's stack does; Black level correction and Area exposure mapping stay.
- **While closed** the section shows only its title and buttons, no summary of the controls set (docs/gui.md).
- **Tracks:** Exposure's spans darktable's soft range, −3 to +4 EV, and a value typed or scripted beyond it, anywhere in the range's ±18 EV, widens it, as darktable's does, until the row is reset; Black level correction's spans darktable's soft range, −0.1 to +0.1, and widens the same way ([Sliders against darktable](#sliders-against-darktable)).
- **Keyboard:** every row is a `SliderRow`, a toggle or a list, so arrows step, Shift steps ten and Alt a tenth, Home and End go to the track's ends, Delete resets, and Enter edits the value. The toggle, egui's checkbox, has Space but no marker or reset yet. The histogram drag (right of the middle for Exposure, left for Black level correction), with the scopes, sends the same `SetControl` commands.

## Automatic mode

darktable's deflicker ([port spec](../ports/exposure.md#automatic-mode-deflicker)): the exposure that puts the Percentile of the raw's histogram at the Target level below the white point, worked out from each photo, so an edit pasted onto a series of frames brings them to one level, a time-lapse's or a bracketed series'.

- **The histogram** is every photosite of the declared image, whatever its colour, its 16-bit value as decoded, before rawprepare. The engine counts it on the GPU once per photo, only while automatic mode is on (decision 87), so manual mode costs nothing (E37). darktable meters rawspeed's crop, which differs on most raws by a few photosites at the edges, moving the exposure by at most 0.012 EV on the corpus ([port spec](../ports/exposure.md#deliberate-differences)).
- **The exposure** is `target − ev`, ev the metered value in EV below rawprepare's white point: Raw black/white point's levels move it, as they move darktable's. It replaces Exposure and both compensations; Black level correction still applies (E35), put 0.01 below the white the computed exposure renders by the operation where it reaches it, the tool binding it in automatic mode within darktable's ±1 alone, since its own bound knows only Exposure's white.
- **On an image that is not a raw** there is no histogram, and automatic mode renders as manual mode (E36).
- **The computed EC readout** shows the exposure applied, reported with the values, before the frame that applies it.

## The picker

darktable's picker on Exposure (`exposure.c:1211-1218`), an area picker of the exposure operation's input, on Exposure's quad, its tooltip darktable's "Sets the exposure adjustment using the selected area." A pick is `Exposure::pick`, darktable's `color_picker_apply` and `_auto_set_exposure` (`:866-973`), which the session applies as one change ([pickers.md](../pickers.md#what-a-pick-writes)):

1. The area's mean RGB, through the input profile's matrix to XYZ D50, darktable's `matrix_in`: colorin's default, the camera's matrix on a colour raw and linear Rec2020 otherwise (`exposure::input_profile`, held to colorin's by a test). To Lab, its a and b set to 0, and back: the grey of its lightness. The input patch is that grey in sRGB, and the readout its L.
2. **Correction:** the target's grey, Y of L = Lightness; the exposure `−log2(Y / Y_target)`, the white floored at 1e-20 as darktable's `white2exposure` floors it, plus the camera's bias where Compensate camera exposure is on and less the held-back exposure where Highlight preservation mode is, which the operation takes off and gives back again, so the area renders at the target. Where it equals Exposure's value nothing is written. Where the black the operation holds is at or above the new white, `2^−e`, Black level correction is written so the black sits 0.01 below it, darktable's `_exposure_set_white`.
3. **Measure:** no control; the lightness the area renders at at the current exposure, held to 0 to 100, kept as the Lightness setting.

- **Bounds:** darktable bounds neither value; the session holds them to their ranges, Exposure's ±18 EV, and writes no exposure that is not finite, a target of 0 %'s, while the patch and L still show the area.
- **Highlight preservation mode** adds its held-back exposure where it is on, in both modes, as darktable's `_auto_set_exposure` does (`:911-913`, `:957-959`).
- **A change of Area mode or Lightness** applies the latest reading again, in correction mode only, as darktable's settings callback does (`:1170-1175`).

## Design choices

- **Exposure's meaning:** darktable's slider, `exposure.exposure` as it is, its default the photo's, so the value shown is darktable's and a move renders as darktable's does (decision 30).
- **Black level's meaning:** an offset over the photo's own black, darktable's black level correction, held within darktable's ±1; a black that reaches the white the photo renders goes 0.01 below it, darktable's margin, so the kernel never divides by zero. A negative one limits how far Exposure brightens, as darktable's black does, so the same Black level means different things on different photos.
- **Compensate camera exposure's default:** the exposure operation's, on for any raw and off for an image that is not one, so its untouched value never changes the render. It is drawn disabled on a photo that records no bias.
- **Decimals shown:** darktable's (`dt_bauhaus_slider_set_digits`), Exposure 3 and Black level correction 4. So Alt's tenth of a step, 0.005 EV and 0.0001, darktable's finest, is sent as shown. A value is rounded to them before it is sent, so a step finer than the row shows sends nothing a reader cannot see.
- **The picker:** declared in the definition, `exposure`, an area picker writing Exposure and Black level and reading the area exposure mapping's settings, named by its slider's quad (`Quad::Picker(id)`), as darktable's page has it and Colour balance RGB's pickers are ([The picker](#the-picker)).
- **Auto's tooltip:** "Sets Exposure so the photo's tones sit around middle grey. A kept Exposure stays as it is."
- **Control ids:** darktable's parameter names, as Colour balance RGB's and RGB primaries' are: `mode`, `compensate_hilite_pres`, `deflicker_percentile`, `deflicker_target_level`. Exposure, Black level correction and Compensate camera exposure have ids of their own, `exposure`, `black_level` and `camera_bias`, which saved edits hold.
- **Automatic mode's parameters:** in the operation's schema since version 2, so its tool's version 3 holds them with no migration: their defaults do not change.
- **Highlight preservation mode's default:** on, as darktable turns it on for a module's first instance and the operation's default is (decision 21), so a Canon raw with its Auto Lighting Optimizer on renders 0.5 EV brighter than before, as darktable renders it. The whole-pipeline comparison still writes darktable's compensation off and the exposure Apertrust applies (decision 62).
- **Mode's sensitivity:** disabled on an image that is not a raw, as darktable's `gui_update` makes it, where darktable also resets the mode to manual in the history; Apertrust leaves the edit as it is, rendering it as manual mode, since an edit pasted onto a series must keep its rule for the raws in it.
- **The area exposure mapping:** Area mode, the patches and Lightness, the picker's settings as `Row::Setting`s kept in settings and the patches as `Row::Patches`, in a group closed at first. darktable keeps its area mode nowhere and puts it back to correction at every `gui_update` (`exposure.c:742`): another photo, undo and redo, a history move, the module's reset. Apertrust keeps it in settings, as it keeps Lightness, a deliberate difference.
- **The computed EC readout:** a computed value's row, the value the session reports from the operation's own computation, signed and with its unit as a slider's value is, as the wireframe shows it; darktable's `%.2f EV` has no plus sign.
- **Edits version 1 saved:** their Exposure, in exact stops, is left out with a warning; no release wrote one (decision 48).

## Phases

- **Built (Phase 2, step 6 of `docs/gui.md`, and the first pass, decision 2):** darktable's page, with rows E0, E3, E4, E6 to E9, E14 and E30 to E32: Mode, Compensate camera exposure, Highlight preservation mode, Exposure, Percentile, Target level, Black level correction and the Area exposure mapping group, with no Advanced controls (decision 29).
- **Built (version 3):** automatic mode, darktable's deflicker, with E32 and E34 to E37 and its computed EC readout (decision 87); the tool's version unchanged, its defaults being too.
- **Built (version 3):** Highlight preservation mode ported, with E33: the held-back exposure read from the maker notes as darktable reads it, given back by default ([port spec](../ports/exposure.md)).
- **Built (Phase 2, decision 79's 79.7):** the picker, with E50 ([The picker](#the-picker)): its pick in correction and measure modes, the Area exposure mapping's settings kept in settings, the patches and the L readout.
- **Later in Phase 2:** Auto with E51.
- **Phase 4:** layers, with E60.
