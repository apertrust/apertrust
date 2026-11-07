# Exposure

Module `exposure`, version 2. It works on layers and has Auto. It drives one operation, `exposure`, which an unedited photo runs, and owns every parameter of it.

It is darktable's exposure module in the first pass (decision 2: rewrite first, improve later). It has darktable's page, every control `gui_init` shows, in its order under its labels, on one untitled tab (decision 29): Mode, Compensate camera exposure, Highlight preservation mode, Exposure, Percentile and Target level in automatic mode, Black level correction, and the Area exposure mapping group. The unported ones are drawn disabled ([Controls](#controls); [README.md](README.md)).

Exposure is darktable's slider, `exposure.exposure` itself, its default the photo's: +0.7 EV on a colour raw, 0 otherwise (decision 30). Black level correction is darktable's offset over the photo's black (decision 22). Contrast and Pivot, colour balance rgb's `contrast` and `grey_fulcrum`, and Brightness, `midtones_Y`, belong to the Colour balance RGB module ([colour-balance-rgb.md](colour-balance-rgb.md); decision 2), whose table holds their rows.

**Edits version 1 saved.** Version 1's Exposure solved for exact stops through the photo's black, so its value means another exposure on every photo. Version 2's migration leaves it out with a warning, its mark with it, and Exposure renders at the photo's default; no release wrote such an edit. Black level correction and Compensate camera exposure keep their values. Contrast and Pivot, which an older version 1 had, are left out with a warning each, the photo editable.

**Status:** built, in `crates/modules/src/exposure.rs`. The app draws it in the Base tab from its definition (`docs/gui.md`, Module panels from definitions), after the Tone equalizer and before Demosaic (decision 38).

- Rows E0, E3, E4, E6 to E9, E14 and E30 to E32 are tests in `crates/modules/tests/exposure.rs`, with the migration's; [Results](#results) gives what they measured.
- A tolerance or a choice marked "proposed" is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

In the first pass Exposure is darktable's page, every control on it in `gui_init`'s order, on one untitled tab with no toggle (decision 29). The Row column gives each control's label and when it shows: darktable shows Compensate camera exposure, Highlight preservation mode and Exposure in manual mode and Percentile and Target level in automatic mode (`Row::When`).

| Control | Row | Kind and range | Default | Changes | Keeps | Binding |
| --- | --- | --- | --- | --- | --- | --- |
| `mode` | Mode, always | Choice: manual; automatic, darktable's deflicker, listed, drawn disabled and refused (it needs the raw histogram) | Manual | How the exposure is set | — | Sets nothing: manual, the only option it can hold, is `exposure.mode`'s default. Drawn disabled on an image that is not a raw, as darktable makes it insensitive. darktable also disables it on a raw that is not one 16-bit channel, a linear DNG, which the engine does not render yet |
| `camera_bias` | Compensate camera exposure, in manual mode | Toggle | The photo's: on for a colour raw | Whether the camera's exposure compensation is undone | Everything, on a photo with no bias | `exposure.compensate_exposure_bias` = the value, set only when it differs from the photo's |
| `compensate_hilite_pres` | Highlight preservation mode, in manual mode | Toggle, not ported: drawn disabled | Off, as the operation leaves it (decisions 21 and 62); darktable turns it on for a new instance | Would add the exposure a camera holds back in its highlight-preserving modes, read from maker notes rawler does not read | — | None: it takes only its default (`Control::unported`), so `exposure.compensate_hilite_pres` stays off |
| `exposure` | Exposure, in manual mode, with darktable's picker, drawn once the picker runs (E50) | Number, −18 to +18 EV, darktable's range, step 0.05, its track over darktable's soft range, −3 to +4 EV (decision 30) | The photo's: +0.7 EV on a colour raw, 0 on a monochrome raw or an image that is not a raw, as darktable's `reload_defaults` sets it | The brightness of every pixel | Contrast and colour: the ratio between any two pixels or channels | `exposure.exposure` = the value, darktable's slider's; the operation subtracts the camera's bias while Compensate camera exposure is on, as darktable's `commit_params` does |
| `deflicker_percentile` | Percentile, in automatic mode | Number, 0 to 100 %, step 1, not ported: drawn disabled | 50 % | Where automatic mode meters the raw histogram | — | None: it takes only its default, so `exposure.deflicker_percentile` stays 50 |
| `deflicker_target_level` | Target level, in automatic mode | Number, −18 to +18 EV, step 0.1, not ported: drawn disabled | −4 EV | Where automatic mode puts that level, below overexposure | — | None: it takes only its default, so `exposure.deflicker_target_level` stays −4 |
| `black_level` | Black level correction, always | Number, −1 to +1, darktable's range, step 0.001, its track over darktable's soft range, −0.1 to +0.1 (decision 30) | 0: the photo's black | Where black sits, as darktable's black level correction | The white point | `exposure.black` = the photo's default (−2⁻¹² for a colour raw) plus the value, within darktable's ±1; a black that reaches the white the photo renders, w = 2^−(e − β), is put 0.01 below it, darktable's margin, so the kernel never divides by zero (decision 30) |

**Area exposure mapping**, last, is darktable's collapsible section, closed at first. It holds darktable's settings for its picker, which no edit stores ([README.md](README.md), rule 12), drawn disabled at darktable's defaults until the picker comes (`Row::ToolSetting`): Area mode, correction or measure, then, under a Target label, Lightness, 0 to 100 %, 50 % by default, as darktable's darktablerc has it.

- The input's and the target's colour patches and the input's L readout come with the picker.
- darktable's computed EC readout, which automatic mode shows under Target level, waits for a row that draws a readout.

**What is not ported.** Automatic is refused as every unported option is (decision 29). Highlight preservation mode, Percentile and Target level are unported controls (`Control::unported`). Each takes only its default:

- a command setting another value is refused;
- an edit holding one is read with it left out, with a warning, so no edit stores a value the render ignores, and nothing needs migrating once they are ported;
- the session reports them unavailable, so they are drawn disabled.

Their parameters stay at darktable's defaults, the only values the operation renders, and the operation refuses any other as not ported (E32).

**Owners.** Exposure owns every `exposure` parameter, darktable's `mode`, `black`, `exposure`, `deflicker_percentile`, `deflicker_target_level`, `compensate_exposure_bias` and `compensate_hilite_pres`. Contrast, Pivot and Brightness are the Colour balance RGB module's `contrast`, `grey_fulcrum` and `midtones_y`, darktable's parameters, and Colour balance RGB owns every `colorbalancergb` parameter (decisions 28 and 32).

The control `exposure` is the operation parameter of the same name, which the module owns. So `apertrust-cli edit --set exposure.exposure=1.7` sets +1.7 EV, 1 EV over a colour raw's default, while `--param exposure.exposure` is refused. A script sends `SetControl { module: "exposure", control: "exposure", value: 1.0, done: true }`, which is the same command a drag ends with.

### Sliders against darktable

Audited against darktable's `src/iop/exposure.c`, its params and `gui_init` ([README.md](README.md), Sliders against darktable). darktable's columns are its own, in the units its slider shows: no slider here has a factor or an offset (Percentile's "%" format gives none, its range being above ±10). `its_sliders_are_darktable_s`, in `crates/modules/src/exposure.rs`, holds the definition to this table and finds no difference (decision 30).

| Slider | darktable: range, soft range | Digits | darktable's step | Apertrust: range, track | Decimals, step | Differs |
| --- | --- | --- | --- | --- | --- | --- |
| Exposure (`exposure`) | −18 to +18 EV; −3 to +4 EV | 3 | 0.05 EV | −18 to +18 EV; −3 to +4 EV | 3; 0.05 EV | No. The value shown is darktable's, +0.700 EV on a colour raw as first shown. A value typed or scripted beyond the soft range widens the track, as darktable's does |
| Percentile (`deflicker_percentile`), not ported | 0 to 100 %; none | 2 | 1 % | 0 to 100 %; the range | 2; 1 % | No |
| Target level (`deflicker_target_level`), not ported | −18 to +18 EV; none | 2 | 0.1 EV | −18 to +18 EV; the range | 2; 0.1 EV | No |
| Black level correction (`black_level`) | −1 to +1, the black itself; −0.1 to +0.1 | 4 | 0.001 | −1 to +1, an offset over the photo's own black (decision 22); −0.1 to +0.1 | 4; 0.001 | No. darktable's slider shows the black, −0.0002 on a colour raw, ours the offset over it, 0 there. The black is held within darktable's ±1, so on a colour raw the control's bottom 2⁻¹² changes nothing, and one that reaches the white the photo renders goes 0.01 below it (decision 30) |
| Lightness (`lightness`), Area exposure mapping's tool setting | 0 to 100 %; none | 1 | 1 % | 0 to 100 %; the range | 1; 1 | No |

Exposure steps by darktable's 0.05 EV, Shift 0.5 EV and Alt 0.005 EV, darktable's coarse and fine steps, growing as a widened track's span does, as darktable's zoom step makes it. A drag and a typed value round to the three decimals shown, as darktable's do. Exposure's range and Black level correction's are darktable's (decision 30); widening a range raises no module version (decision 43).

### Tooltips

These are the tooltips (`module-exposure-control-<control>-promise`), each turned into the rows named. Exposure, Highlight preservation mode, Percentile and Target level have darktable's tooltips; Mode, which has none in darktable, has words of the same kind. Nothing of theirs is ported but what E32 holds.

| Control | Tooltip | Rows |
| --- | --- | --- |
| Exposure | Adjusts the exposure correction. | E4, E6 to E9 |
| Black level correction | Sets which level is black. Keeps white. 0 = untouched. | E30 |
| Compensate camera exposure | Undoes the exposure compensation set on the camera. | E31 |
| Mode, Highlight preservation mode, Percentile, Target level | darktable's | E32 |

### Transfer functions

- **Exposure** has none: `exposure.exposure` is the control's value, darktable's slider's. The operation scales `(in − b)` by `1 / (2^−(e − β) − b)`, with e the value, b the black and β the camera's bias while Compensate camera exposure is on, `clamp(bias, −5, +5)` as darktable's `_get_exposure_bias` trusts it. The image context carries the bias unclamped and `apertrust-modules` cannot call the exposure crate, so `camera_bias` repeats the clamp, and a test holds it to the operation's. So a move of v EV from the photo's default scales every pixel by (w₀ − b) / (w − b), w₀ and w the whites the default and the value render: 2^v where b is 0, slightly less over a colour raw's black of −2⁻¹² (+3.991 EV for +4 EV with no bias), as in darktable. A negative Black level caps the factor at 1 / −b, as darktable's black does. The default is `default_exposure`, darktable's `reload_defaults` restated, which a test holds to the operation's own.
- **`black_level(v)`** (`BlackLevel`): b₀ + v, held within darktable's ±1, with derivative 1 inside; a black at or above the white the photo renders, w = 2^−(e − β) (`rendered_white`), is put at w − 0.01, as darktable's `gui_changed` puts it, with derivative 0. darktable compares with the white of its slider's exposure; this compares with the white the kernel divides by, the camera's bias included. On a colour raw the control's bottom 2⁻¹² holds the black at −1. At the photo's default exposure the white is 0.62 with no bias, 0.39 at −0.67 EV of bias and 0.019 at −5 EV. The photo's own black, 0 or below, never reaches a white, so the black is bound only while Black level is set. The margin has a cost: the kernel computes 2^−e − b, and there the GPU's `exp2` error is amplified up to about 100×, estimated at 2e-4 relative at worst; E30 measures it.

## Invariants

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| E0 | None: every control at its default | | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | Every image |
| E3 | Each control, moved and reset | Exposure +1, Black level correction +0.01, Compensate camera exposure off, and the three together; then each reset by double-click, Delete and the header's reset | `bind`; frame | As E0 | Exact | Ramp; 90D |
| E4 | Exposure | The photo's default −4, −1, −0.01, +0.01, +1 and +4 EV; −18 and +18, the range's ends | Module output: each pixel's RGB over E0's | One factor for every pixel and channel, so contrast and colour are kept | Spread ≤ 1e-5 relative, on values over 1e-6 (proposed) | Ramp, sweep; 90D, Z 6, X-T5 |
| E6 | Exposure, across a series | The photo's default +1 EV on photos with biases 0 and −0.67 EV | E4's factor at the scene, in EV | The same on both, so a pasted Exposure means the same on every photo | 0.01 EV (proposed) | Ramp in both contexts; 90D and Z 6 |
| E7 | Exposure | −18, the default ±4 and +18 EV | Frame, on the extended ramp | Finite; a brighter grey never renders darker | Exact order, no NaN or infinity | Extended ramp; 90D |
| E8 | Exposure | E4's values | Module output: E4's factor | darktable's, (w₀ − b₀) / (w − b₀), at biases 0, −0.67 and −5 EV | 1e-5 relative (proposed) | Ramp with biases 0, −0.67 and −5 EV; 90D, Z 6, X-T5 |
| E9 | Exposure and Black level correction | The golden `exposure` case's values, as darktable's sliders show them | The bound parameters; the kernel's inputs | darktable's parameters, and the inputs the golden harness checks against darktable's render | Exact, bit for bit | None: an image that is not a raw, and a colour raw with its bias not compensated |
| E14 | Exposure with another control | Exposure +1, then Black level correction or Compensate camera exposure; and the other way round | Frame | The same whichever was set first | Exact | Ramp; 90D |
| E30 | Black level correction | −1, −0.1, −0.01, +0.01, +0.1, +1: the range's ends and the track's | Module output, on the ramp | The level set as black renders 0; the grey Exposure renders at 1.0 stays at 1.0; greys between stretch linearly. Where a value puts black at or above the white the photo renders, black goes 0.01 below it (decision 30): at +1 on photos with a bias up to +0.7 EV, at +0.1 on photos with a bias below −2.6 EV; at −1 on a colour raw it stops at −1, darktable's lowest. The margin amplifies the GPU's `exp2` error, which this row measures there | 1e-5 relative (proposed) | Ramp; 90D |
| E31 | Compensate camera exposure | Off, on a raw with bias −0.67 EV; off, on a raw with none | Module output: E4's factor | Darkens by the bias, 0.67 EV; with no bias, changes nothing | 0.02 EV (proposed); exact | Ramp in both contexts; 90D, Z 6 |
| E32 | What is not ported | Highlight preservation mode on; Percentile 0 and 100; Target level −18 and +18; Mode automatic; each beside Exposure +1, as a command and pasted into a saved edit | The edit | The command is refused and the edit stays as it was; the pasted value is left out with one warning, Exposure +1 kept, so the edit is the one without it | Exact | None: no render, since the edit never holds one |
| E50 | The picker, later | A pick on a flat patch | Scene: the patch's luminance | Becomes middle grey, RGB 0.18; only Exposure changes | 0.01 EV (proposed) | Step chart |
| E51 | Auto, later | Auto, with Exposure kept and not kept | Values, marks and history | Moves only Exposure, and only if it is not kept; marks it Auto; one undo reverts it | Exact | Ramp |
| E60 | Each control on a layer, Phase 4 | E4's and E30's values on a full-mask layer at 100 % | Scene, inside the mask | Changes the image as the same values do on the background | As the row it repeats | Ramp |

E50 and E51 wait for an open architecture question: how a picked area or Auto's statistic is read back while reductions stay on the GPU (`docs/engine.md`, Open questions, Auto).

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/exposure.rs`; the synthetic rows run on CI's software GPUs too. The corpus rows run with `APERTRUST_RAW_CORPUS`, the chart's with `APERTRUST_TEST_IMAGES`, both `#[ignore]`d. Where a row fails, its proposed bound stays in an ignored test, open, and CI checks the parts of it that hold; the bound itself is unchanged.

**Where the rows read.** E4, E8, E30 and E31 measure Exposure's own output, the image leaving the exposure operation (the module output, `docs/modules/README.md`). These rows are about Exposure's maths, darktable's exact multiply. Colour calibration, darktable's default, runs between exposure and AgX on every colour raw (decision 27) and does not keep them at the scene: it raises values below 2⁻¹⁶, clips negative RGB to 0 and compresses colours beyond the gamut, none of which scales with the image.

At the scene, with the synthetic images balanced as below:

- E4 holds on the ramp (a spread of 2.6e-6) and fails on the sweep (1.1e2, its colours beyond the gamut) and on the 90D, the Z 6 and the X-T5, where colour calibration does not scale with the image.
- E30 fails: relative errors up to 1.0, where Black level puts a grey below black and colour calibration clips it to 0, and 3.9e-4 between.
- E31 holds, a spread of 2.3e-6.

At Exposure's output all three hold, as below.

A synthetic image is already white balanced, so the tests' colour calibration adapts it from the pipeline's own illuminant, D50, not the camera's (README, Test images). The camera's would balance it twice, turning the ramp's greys magenta and, at the frame, failing E7 at −4 EV, green clipping to 0 in the shadows.

| Row | Result |
| --- | --- |
| E0 | Holds, exactly. `bind` sets nothing with every control written at its default, or kept there, Mode, Highlight preservation mode, Percentile and Target level included. Frames byte-identical to no edit on the ramp and the sweep at biases 0 and −0.67 EV, and on a small synthetic Bayer raw through the export and the viewport pipes, each on one pipeline that colour balance rgb joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| E3 | Holds, exactly. Exposure +1, Black level +0.01, Compensate camera exposure off and the three together, each reset control by control (`ResetControl`) and by the module's reset (`ResetModule`), on one pipeline that colour balance rgb joins and leaves between the moves, on the ramp and the synthetic raw's viewport; on the 90D through the viewport and the export. |
| E4 | Holds, at Exposure's output: spreads of at most 2.0e-7 on the ramp and 2.3e-7 on the sweep, the range's ends included; at most 2.4e-7 on the 90D, the Z 6 and the X-T5, with colour calibration on every colour raw |
| E6 | Holds: the default +1 EV gives factors 3.4e-4 EV apart at biases 0 and −0.67 EV, darktable's black weighing the two photos' whites differently |
| E7 | Holds at −18, the default ±4 and +18 EV: every frame finite, no grey darker than a darker one |
| E8 | Holds: within 9.5e-8, 1.2e-7 and 2.4e-7 relative of darktable's factor at biases 0, −0.67 and −5 EV on the ramp; within 3.3e-7 on the 90D, the Z 6 and the X-T5 at every value, −18 and +18 EV included |
| E9 | Holds, exactly: the case's +1 EV and black 0 bind darktable's parameters and the kernel's inputs bit for bit, on an image that is not a raw and on a colour raw |
| E14 | Holds, exactly: one pipeline sets the first control and then both, another the second and then both, for Exposure with Black level and with Compensate camera exposure, on the ramp and the 90D (viewport and export) |
| E30 | Holds, at Exposure's output: within 5.1e-7 relative at −1, −0.1, −0.01, +0.01, +0.1 and +1, at biases 0, −0.67 and −5 EV, the black held at −1 included; where the black goes 0.01 below the white, 1.3e-7, 1.3e-7 and 4.1e-7 at biases 0, −0.67 and −5 EV. lavapipe and WARP wait for CI |
| E31 | Holds, at Exposure's output: off darkens by 0.6697 EV on a −0.67 EV raw, one factor for every pixel within a spread of 1.8e-7; with no bias, exactly nothing changes |
| E32 | Holds, exactly: each command refused with the edit unchanged, each pasted value left out with one warning |

## In the panel

- **Compensate camera exposure is drawn disabled** on a photo that records no bias, as an action that cannot run is (`ui-conventions`), with the tooltip naming the camera's bias as 0. Built through `Module::available`; the tooltip still says what the control does, since a disabled control's tooltip cannot say why yet (`docs/gui.md`, the renderer's gaps). darktable's labels add the bias, "compensate camera exposure (+0.7 EV)" and "highlight preservation mode (0.0 EV)"; a label with a value of the photo's waits for the renderer.
- **Mode** is a list, its automatic option drawn disabled, so the list reads as darktable's and says what is missing. The whole row is disabled only on an image that is not a raw; darktable also disables it on a raw that is not one 16-bit channel, which the engine does not render yet.
- **The unported controls:** Highlight preservation mode, Percentile and Target level are drawn disabled. Percentile and Target level show only in automatic mode, so never yet. darktable shows Highlight preservation mode only on a photo whose maker notes record a bias; since rawler reads none, it is shown disabled on every photo, as a control that cannot act is (decision 30).
- **While closed** the section shows only its title and buttons, no summary of the controls set (docs/gui.md).
- **Tracks:** Exposure's spans darktable's soft range, −3 to +4 EV, and a value typed or scripted beyond it, anywhere in the range's ±18 EV, widens it, as darktable's does, until the row is reset; Black level correction's spans darktable's soft range, −0.1 to +0.1, and widens the same way ([Sliders against darktable](#sliders-against-darktable)).
- **Keyboard:** every row is a `SliderRow`, a toggle or a list, so arrows step, Shift steps ten and Alt a tenth, Home and End go to the track's ends, Delete resets, and Enter edits the value. The toggle, egui's checkbox, has Space but no marker or reset yet. The histogram drag (right of the middle for Exposure, left for Black level correction), with the scopes, sends the same `SetControl` commands.

## Design choices

- **Exposure's meaning:** darktable's slider, `exposure.exposure` as it is, its default the photo's, so the value shown is darktable's and a move renders as darktable's does (decision 30).
- **Black level's meaning:** an offset over the photo's own black, darktable's black level correction, held within darktable's ±1; a black that reaches the white the photo renders goes 0.01 below it, darktable's margin, so the kernel never divides by zero. A negative one limits how far Exposure brightens, as darktable's black does, so the same Black level means different things on different photos.
- **Compensate camera exposure's default:** the exposure operation's, on for any raw and off for an image that is not one, so its untouched value never changes the render. It is drawn disabled on a photo that records no bias.
- **Decimals shown:** darktable's (`dt_bauhaus_slider_set_digits`), Exposure 3 and Black level correction 4. So Alt's tenth of a step, 0.005 EV and 0.0001, darktable's finest, is sent as shown. A value is rounded to them before it is sent, so a step finer than the row shows sends nothing a reader cannot see.
- **The picker:** in the definition, as darktable's page has it and Colour balance RGB's pickers are; the renderer draws no quad until the picker runs (E50).
- **Auto's tooltip:** "Sets Exposure so the photo's tones sit around middle grey. A kept Exposure stays as it is."
- **Control ids:** darktable's parameter names, as Colour balance RGB's and RGB primaries' are: `mode`, `compensate_hilite_pres`, `deflicker_percentile`, `deflicker_target_level`. Exposure, Black level correction and Compensate camera exposure have ids of their own, `exposure`, `black_level` and `camera_bias`, which saved edits hold.
- **The parameters not ported:** in the operation's schema at darktable's defaults, schema version 2, so Exposure owns them all; any other value is refused as not ported. Highlight-preservation compensation's default is off, as the operation renders and the whole-pipeline test sets darktable (decisions 21 and 62). darktable's on for a new instance would make the one value rendered the refused one.
- **Their controls' values:** unported controls (`Control::unported`). They take only their defaults and are refused otherwise, as an unported option is; the session reports them unavailable, so they are drawn disabled, and the operation's refusal never meets an edit.
- **Mode's sensitivity:** disabled on an image that is not a raw, as darktable's `gui_update` makes it; live on a raw, so its list shows automatic disabled. darktable also disables it on a raw that is not one 16-bit channel, which the engine does not render yet.
- **The area exposure mapping:** Area mode, a Target label and Lightness, the picker's settings, as `Row::ToolSetting`s in a group closed at first. The patches and the L readout, which are no setting, wait for the picker.
- **The computed EC readout:** left out until a row draws a readout; it shows only in automatic mode, which cannot be chosen.
- **Edits version 1 saved:** their Exposure, in exact stops, is left out with a warning; no release wrote one (decision 48).

## Phases

- **Built (Phase 2, step 6 of `docs/gui.md`, and the first pass, decision 2):** darktable's page, with rows E0, E3, E4, E6 to E9, E14 and E30 to E32: Mode, Compensate camera exposure, Highlight preservation mode, Exposure, Percentile, Target level, Black level correction and the Area exposure mapping group, with no Advanced controls (decision 29).
- **Later in Phase 2:** the picker and Auto, with E50 and E51, once the readback question is decided.
- **Phase 4:** layers, with E60.
