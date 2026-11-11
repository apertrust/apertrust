# Tone equalizer

Module `toneequal`, version 1. It works on layers and has no Auto.

It is darktable's tone equalizer in the first pass (decision 2). It has darktable's page: the Simple, Advanced and Masking tabs as content tabs, the controls in `gui_init`'s order under darktable's labels in British spelling, and the display of the exposure mask under every tab (decision 29).

It drives one operation, `toneequal` ([../ports/toneequal.md](../ports/toneequal.md)), at darktable's place, camera RGB 24.0 (decision 22). An unedited photo does not run it: it is planned only while a control is off its default, as in darktable, where the tone equalizer is off until the user turns it on. The module owns every one of the operation's eighteen parameters; none is fixed.

**Status:** built, in `crates/modules/src/tone_equalizer.rs`, its Advanced tab's graph of the bands included but for the mask's histogram ([Gaps](#gaps), 1). The app draws it from its definition (`docs/gui.md`, Module panels from definitions) in the Base tab, after the Tone curve and before Exposure, in [README.md](README.md)'s order (decision 38).

- Its rows are tests in `crates/modules/tests/tone_equalizer.rs`; [Results](#results) gives what they measured.
- The operation ports every luminance estimator and three of darktable's five filters: no filter, EIGF and averaged EIGF. The guided filter and the averaged guided filter are listed and drawn disabled until their follow-up (the spec's Planned).
- The [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Planned: a switched module** (decision 32). The Tone equalizer becomes switched: off until a control changes, then on with darktable's defaults for every control not set, until its reset turns it off ([README.md](README.md), rule 4).

- Its defaults are darktable's already, so its controls and binding stay as they are.
- It becomes version 2: a version 1 edit migrates on where a value was off its defaults, its values otherwise dropped.
- Its rows become a switched module's ([README.md](README.md), Rows every table has). Q0 becomes off; Q1 measures what it measures today, darktable's ripple, as the module on and untouched; Q2's resets follow the switched rows; new rows after Q5 hold on at darktable's defaults and version 1's edits.

The rest of this page describes the module as built.

## Controls

Each control is one `toneequal` parameter, shown as darktable's slider shows it. The bands, the two compensations and the mask's settings are shown as they are. The curve's σ is shown as darktable's Curve smoothing slider shows it, `s` with σ = √2^(1 + s); the binding computes σ as darktable's `smoothing_callback` does, in f64, rounded once to f32.

Each slider is darktable's ([Sliders against darktable](#sliders-against-darktable)):

- Its range is darktable's hard range in the unit shown, what a value may hold.
- Its track spans darktable's soft range, widening to show a value beyond it until the row is reset, as darktable's does.
- Its step is darktable's, what `dt_bauhaus_slider_get_step` gives over the soft range, and 1 for Filter diffusion, a whole number.
- Every slider shows two decimals, as darktable's do, and Filter diffusion none.

Control ids are darktable's parameter names, which below −2 EV are not the bands' labels: `shadows` is the −4 EV band, labelled "mid-tones" in darktable's own words (`docs/ports/toneequal.md`, Parameters).

| Tab | Control | Parameter | Range | Soft range (darktable's slider) | Step | Default | Shows |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Simple | −8 EV, −7 EV, … −1 EV, +0 EV (`noise`, `ultra_deep_blacks`, `deep_blacks`, `blacks`, `shadows`, `midtones`, `highlights`, `whites`, `speculars`) | the same | −2 to +2 EV each | the range | 0.01 EV | 0 | Always |
| Advanced | Curve smoothing (`smoothing`) | `smoothing`, σ = √2^(1 + s) | −2.33 to 1.67, so σ 0.6306887 to 2.5227547 | −1 to 1, σ 1 to 2 | 0.01 | 0, σ = √2 | Always |
| Advanced | Mask exposure compensation (`exposure_boost`) | `exposure_boost` | −16 to +16 EV | ±4 EV | 0.05 EV | 0 | Always |
| Advanced | Mask contrast compensation (`contrast_boost`) | `contrast_boost` | −16 to +16 EV | ±2 EV | 0.01 EV | 0 | With the guided filter or EIGF, chosen on the Masking tab, as darktable shows it; not with No nor the averaged filters, which ignore it |
| Masking | Luminance estimator (`method`) | `method` | RGB average, HSL lightness, HSV value / RGB max, RGB sum, RGB euclidean norm, RGB power norm, RGB geometric mean, all ported | — | — | RGB euclidean norm | Always |
| Masking | Preserve details (`details`) | `details` | No, Averaged guided filter, Guided filter, Averaged EIGF, EIGF; the two guided filters not ported, drawn disabled and refused | — | — | EIGF | Always |
| Masking | Filter diffusion (`iterations`) | `iterations`, the nearest whole number | 1 to 20 | 1 to 5 | 1 | 1 | With a filter: any but No |
| Masking | Smoothing diameter (`blending`) | `blending` | 0.01 % to 100 % | 1 % to 45 % | 0.5 % | 5 % | With a filter |
| Masking | Edges refinement/feathering (`feathering`) | `feathering` | 0.01 to 10000 | 0.1 to 50 | 0.5 | 1 | With a filter |
| Masking | Mask quantisation (`quantization`) | `quantization` | 0 to 2 EV | the range | 0.01 EV | 0 | With a filter |
| Under every tab | Display exposure mask | none: a view mode, darktable's toggle, which no edit stores (rule 10) | toggle | — | — | Off | Always; drawn disabled until the mask preview comes |

### Sliders against darktable

Audited against darktable's `src/iop/toneequal.c`, its params and `gui_init` ([README.md](README.md), Sliders against darktable). darktable's columns are its own, in its parameters' units: no slider has a factor or an offset (Smoothing diameter's "%" format gives none, its range being above ±10). `its_sliders_are_darktable_s`, in `crates/modules/src/tone_equalizer.rs`, holds the definition to this table and finds no difference.

| Sliders | Range | Soft range | Digits | Step |
| --- | --- | --- | --- | --- |
| The nine bands, −8 EV (blacks) to 0 EV (speculars), " EV" | −2 to 2 EV | none | 2 | 0.01 EV |
| Curve smoothing, a slider of no parameter, `smoothing` = √2^(1 + value) | −2.33 to 1.67 | −1 to 1 | 2 | 0.01 |
| Mask exposure compensation (`exposure_boost`), " EV", with Auto's wand | −16 to 16 EV | −4 to 4 EV | 2 | 0.05 EV |
| Mask contrast compensation (`contrast_boost`), " EV", with Auto's wand | −16 to 16 EV | −2 to 2 EV | 2 | 0.01 EV |
| Filter diffusion (`iterations`), a whole number | 1 to 20 | 1 to 5 | 0 | 1 |
| Smoothing diameter (`blending`), "%" with no factor, its range above ±10 | 0.01 to 100 % | 1 to 45 % | 2 | 0.5 % |
| Edges refinement/feathering (`feathering`) | 0.01 to 10000 | 0.1 to 50 | 2 | 0.5 |
| Mask quantization (`quantization`), " EV" | 0 to 2 EV | none | 2 | 0.01 EV |

Filter diffusion's step is the 1 `dt_bauhaus_slider_from_params` gives an int; every other step is `dt_bauhaus_slider_get_step`'s over the soft range, or the range where there is none, since no float from params is given one (`testing::DarktableSlider::whole`).

**Curve smoothing's ends.** darktable's slider runs from −2.33 to 1.67, σ = √2^(−1.33) = 0.6306887 to √2^2.67 = 2.5227547, which are the operation's schema's limits exactly (`docs/ports/toneequal.md`, Parameters), so the binding needs no clamp. It computes `powf(M_SQRT2_F, 1.0f + s)` in f64 and rounds it once to f32, so every platform binds the same σ, the correctly rounded one. A value set where darktable's `gui_update` shows a stored σ, `logf(σ) / logf(√2) − 1`, binds σ back within one f32 step (Q4).

The promises, the tooltips, are darktable's in British English. darktable gives a band none, so a band's says which tones it corrects in darktable's words for them ("blacks" for −8 EV, "mid-tones" for −4 EV). Its full name, what screen readers say, adds them to the label ("−4 EV, mid-tones"). A script sends `SetControl { module: "toneequal", control: "shadows", value: 0.5, done: true }`, and `apertrust-cli edit --set toneequal.shadows=0.5` does the same: half a stop more for the −4 EV band.

**Presets:** none yet. darktable's thirteen (simple tone curve; mask blending for all purposes and for people with backlight; compress shadows and highlights with EIGF, strong, medium or soft; contrast tone curve, soft, medium or strong; relight: fill-in) come with the header's presets menu (Phase 3); its three with the guided filter wait for that filter.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has). No control's unit is Apertrust's: curve smoothing is darktable's own slider for σ, an exact change of coordinates, and Q4 holds the module's values to darktable's parameters through it.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| Q0 | None: every control at its default, written at it or kept there | | `bind`; the plan's instances and cache keys; the frame, on one pipeline that the tone equalizer joins and leaves on the way | `bind` returns nothing; instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| Q1 | The first step | `toneequal` planned at its defaults, the operation's (every band 0, EIGF on the euclidean norm), as any control's first move off its default plans it | The tone equalizer's output over its input on the ramp, in EV; the frame, ΔE2000 against Q0's: all of it, inside the gamut and within 80 % of its saturation | darktable's behaviour: every pixel scaled by the zero bands' curve, darktable's ripple, −0.013 to +0.025 EV at the default smoothing (`docs/ports/toneequal.md`, Setup), as darktable's first step does | The frame measured and reported, without a bound (decision 22); the correction within the zero bands' curve's range, 5e-5 relative, the f32 rounding the port's own GPU test allows its curve | Ramp, sweep; the chart; every corpus raw |
| Q2 | Each control, moved and reset | The −4 EV band +1 EV; −8 EV −0.5 with +0 EV +0.75; Curve smoothing 0.5 with −5 EV +0.5; Mask exposure −1 EV with Mask contrast +0.5 EV; Preserve details No on the RGB power norm; Averaged EIGF with Filter diffusion 2, Smoothing diameter 3 %, Edges refinement 3 and Mask quantisation 0.5 EV; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | `bind`; the frame | Changes when moved; as Q0, the tone equalizer leaving the plan, once reset | Exact | Ramp; the small raw through the viewport; 90D through the viewport and the export |
| Q3 | Two controls, in either order | The −4 EV band +1 EV and Mask exposure −1 EV; Preserve details No and the RGB power norm | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| Q4 | Every control, the darktable match | The values of darktable's tests 0080, 0129, 0131, 0133 and 0134 and the two cases edited from them, the golden cases `toneequal-eigf`, `-average`, `-max`, `-power`, `-geomean`, `-smoothing` and `-eigf-iterations`, as darktable's sliders show them, Curve smoothing as darktable's `gui_update` shows σ | The parameters the module binds, against those the golden harness renders each case with (`golden::ports::plan_values`); and through the operation's setup, the kernel's inputs and its curve | darktable's parameters, and so the inputs the golden harness checks against darktable's render | Exact, but σ within two f32 steps; the curve within 1e-6 relative | The seven cases' parameters, on the CPU |
| Q5 | Each control on a layer, Phase 4 | Q2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As Q2 | Ramp |

Q4 is how the module matches darktable. The golden cases check the operation against darktable-cli at the cases' settings, ΔE2000 means of 0.0002 to 0.0004 and maxima of at most 0.0015 (`docs/ports/toneequal.md`, Results). Q4 holds the module's controls, set to those settings, to bind those parameters.

Its first step, Q1, is the operation's curve with every band at 0, the same in every mask setting, which the port holds to darktable's restated f32 solve within 2e-5 EV (`every_band_at_zero_ripples_as_darktables`).

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/tone_equalizer.rs`. The synthetic rows and Q4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS` and the chart's with `APERTRUST_TEST_IMAGES`, both `#[ignore]`d.

| Row | Result |
| --- | --- |
| Q0 | Holds, exactly. `bind` sets nothing with every control written at its default, or kept there. Frames byte-identical to no edit on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that the tone equalizer joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| Q1 | darktable's ripple, every tone brightened or darkened by up to 1.8 %; ΔE2000 max 0.481, no pixel over 1.0 on any image (below) |
| Q2 | Holds, exactly, on the ramp and on the small raw through the viewport, and on the 90D through the viewport and the export: every move changes the frame, and resets render as no edit |
| Q3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| Q4 | Holds, exactly, on all seven cases: the parameters bound, the kernel's uniforms and the curve are the cases' (below) |

Q1 in detail, the first step as darktable's tone equalizer switched on at its defaults gives it:

- **The ramp's greys** are each scaled by darktable's zero-band curve: −0.0135 to +0.0253 EV, the curve's own range at the default smoothing, as the setup's f64 curve gives it.
- **The frame:** the ramp a mean ΔE2000 of 0.0297, max 0.319. The sweep a mean of 0.0511, max 0.457, none over 1.0, the same inside the gamut and within 80 % of its saturation (means 0.0526 and 0.0528), as the step is one gain per pixel whatever its colour.
- **The chart:** a mean of 0.1397, max 0.480.
- **The 31 corpus raws:** means of 0.0484 (Pentax K-3 Mark III) to 0.2266 (Canon EOS 7D Mark II), maxima of 0.254 to 0.481, no pixel over 1.0 on any.

Q4 in detail: the parameters bound are the cases' bit for bit, curve smoothing's round trip through darktable's slider included (σ = √2 reads 0 and binds nothing; 1.2 reads −0.47393 and binds 1.2 back). So the kernel's uniforms, the whole-image passes' among them, and the curve are the cases' exactly.

## In the panel

- **Tabs:** Simple, Advanced and Masking, the first shown at first; the tab shown is the app's for the run. Display exposure mask shows under every tab, after its rows, drawn disabled.
- **Simple** is darktable's nine sliders, −8 EV at the top, each filled from 0.
- **Masking:** Luminance estimator and Preserve details are dropdowns with every darktable option, the two guided filters drawn disabled; Filter diffusion, Smoothing diameter, Edges refinement/feathering and Mask quantisation follow Preserve details at once, hidden while it is No, as darktable hides them.
- **Advanced:** darktable's graph of the bands first ([below](#the-bands-graph)), then Mask contrast compensation, which follows Preserve details from the Masking tab: shown with EIGF and hidden with No or Averaged EIGF, as darktable's is.
- **Tracks** span darktable's soft ranges, and widen to show a value typed or scripted beyond one, as darktable's do, until the row is reset.
- **Keyboard:** every slider row is a `SliderRow`, stepping by darktable's step, Shift ten steps and Alt a tenth, the value rounded to the decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

### The bands' graph

The Advanced tab opens with darktable's graph of the bands: `Row::Custom` with `Widget::Equalizer` over the nine bands, drawn by the design system's `Equalizer` (`docs/gui.md`, The equalizer). It is a second view of the bands that stores nothing (decision 2).

**What it draws:**

- −8 EV to +0 EV across, +2 EV to −2 EV down, the black-to-white bars at its start and under it, and each band's node with its bar from the ground.
- The curve the operation renders through the bands (`Module::curve`, decision 31), from the fit `apertrust-color` holds for the operation's setup and the graph alike (`tone_equalizer::Curve`, decision 65). So the curve drawn is the curve rendered, exactly.
- A flat curve at 0 EV while every control is at its default, since the module then plans no operation.
- The curve in the warning colour where the fit fails, which over the smoothing range it never does.

**A drag** anywhere on it moves the bands around the pointer's exposure, as darktable's does:

1. Each step of the pointer raises every band by its rise (the graph's height being 4 EV) times a Gaussian of the band's distance from the exposure the pointer left. The Gaussian's standard deviation is σ²/2, σ the curve's smoothing (`Module::curve_width`), with no limit.
2. Then every band is set to the correction at its node of the curve those bands give (`Module::curve` again), which keeps it within ±2 EV, as darktable's `set_new_params_interactive` and `pixel_correction` do.

So after a drag the nodes sit on the curve. +1 EV at −4 EV from flat bands sets 0.96, 0.65, 0.09 and 0.04 EV from the −4 EV band out, and −0.01 at the ends, where the falloff alone gives 1, 0.61, 0.14, 0.01 and 0.0003. Under the pointer, a ring of radius σ/√2 on the curve shows how far a drag reaches, and the node in reach is filled.

Every change while the pointer moves sets the bands whose value, rounded to the two decimals their sliders show, changes. The release, or the pointer leaving the graph, ends the gesture on each band it moved, as darktable's `area_enter_leave_notify` commits the drag, the bands' `done: true` writes sent together as one `Step { source: You }` (decision 78; [history.md](../history.md#what-makes-a-step)), so the drag is one history step. The pointer's last move before the release counts, as darktable's last motion does. A double-click resets the nine bands, as its tooltip, darktable's, says: nine `ResetControl`s sent as one `Step`, one history step.

**By keyboard** the graph is one focus stop:

- Page Up and Page Down choose a band, Home and End the first and the last.
- The arrows move it by 0.01 EV: Shift ten, Alt a tenth, but never less than the 0.01 EV its slider shows, as a slider row's keys.
- Delete resets it and Shift+Delete all nine, as one `Step`.
- Each band is a slider for screen readers, named as its own slider is.

**The mask's histogram** behind the bands, with darktable's warnings where the mask overflows them, waits for the engine's `mask_histogram` readout (decision 19). Until it comes the graph draws none and no warning.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The bands' graph's histogram.** The graph is built ([In the panel](#in-the-panel)), but the histogram of the luminance mask behind it, with darktable's warning signs where the mask overflows −8 or 0 EV, waits for the engine. The histogram is decided (decision 19; `docs/engine.md`, Readouts): the operation's `mask_histogram` readout, its 256 bins over −8 to 0 EV and its 5th and 95th percentiles, which the definition is to declare (`Definition::readouts`) and the graph's row to name. The widget takes them already (`MaskHistogram`: the bins, their range in EV and the two percentiles), and the renderer hands it `None` until the readout comes. darktable's graph scales its histogram to `max_histogram`, which only grows until the mask's settings change; the widget scales each reading to its own tallest bin, which the readout's task may revisit.
2. **The wands** on Mask exposure compensation and Mask contrast compensation, darktable's "auto-adjust the average exposure" and "auto-adjust the contrast": they solve for the compensations from the mask histogram's percentiles, which come with the `mask_histogram` readout (decision 19), and the definition's `Quad` has no Auto (wand) yet, nor is it settled how an Auto turns a reading into values (`docs/engine.md`, Open questions, Auto). Left out.
3. **The showmask button.** darktable draws Display exposure mask as a toggle with the showmask icon; it is drawn as a checkbox row, disabled, a `Row::ToolSetting`, since the mask preview, a view mode (`docs/ports/toneequal.md`, Planned), is not built and `Quad` has no showmask.
4. **The on-photo editing** darktable offers with the module focused: the cursor's readout of the mask's exposure, and scrolling over the photo to raise or lower the tones under it, which writes a smooth bump into the bands (`docs/ports/toneequal.md`, Tool binding notes). Both need on-photo tools (decision 16); the cursor's readout will be a readout with an argument (decision 19).
5. **The module's tooltip,** darktable's "relight the scene as if the lighting was done directly on the scene": a definition implies no message for the module's own tooltip. Nor does darktable's message when the curve's fit fails, "the interpolation is unstable, decrease the curve smoothing", which the f64 fit never meets over the smoothing range; were it to fail, the graph would draw the bands joined by straight lines in the warning colour, and the operation would refuse to prepare.

## Design choices

- **Control ids:** darktable's parameter names, the bands' `noise` to `speculars` among them, so `--set` and scripts name what darktable's history names, as Colour balance RGB's and RGB primaries' do.
- **Curve smoothing:** stored as darktable's slider shows it, `s`. The binding computes σ = √2^(1 + s) in f64, rounded once to f32, so every platform binds the same σ; the schema's limits are the slider's ends exactly, so no clamp is needed.
- **Filter diffusion:** a number from 1 to 20 shown without decimals, bound as the nearest whole number, darktable's integer field. The definition format has no whole-number kind of control.
- **The bands' labels and full names:** darktable's labels, "−8 EV" to "+0 EV", with its minus sign U+2212. The full name adds darktable's word for the tones, its `$DESCRIPTION` ("−8 EV, blacks"). The promise says which tones, since darktable has no tooltip.
- **Display exposure mask:** a `Row::ToolSetting` toggle under every tab (`Definition::after`), drawn disabled at off. It is no control, since no edit stores a view mode (rule 10).
- **Mask contrast compensation:** a `Row::When` on the Advanced tab following Preserve details on the Masking tab, for the guided filter and EIGF, as darktable's `show_guiding_controls` has it. `Definition::check` lets a `When` follow a control another tab shows outside any group or mode.
- **Row ids:** Q, for tone eQualizer, since E, C, B, P, T, W and H are taken.
- **The graph's row:** `Row::Custom` with `Widget::Equalizer` over the nine bands, as decision 19 has a custom row carry a readout. It is a second view of controls that have rows of their own, which `Definition::check` allows beside them: one equalizer a control at most, over number controls, the equalizer the only widget allowed so until another needs it. The curve is `Module::curve` of the first band, and the drag's reach `Module::curve_width`, a provided method.
- **The graph while the module is untouched:** a flat curve at 0 EV. The module plans no operation, so nothing is corrected, and the first step brings darktable's ripple, Q1. darktable's ripple curve would draw what does not render (decision 31).
- **A drag's step:** darktable's: the bands raised by its falloff at the exposure the pointer left, with no limit, then each set to the correction at its node of the curve they give, which keeps it within ±2 EV. So a drag depends, as darktable's does, on how many steps the pointer took. The widget keeps the bands unrounded through the gesture, and the renderer rounds what it sends to the two decimals the sliders show. The falloff alone would leave the nodes off the curve, by up to 0.05 EV at the default smoothing and 0.24 EV at a smoothing of −1.
- **A fit that fails:** the bands joined by straight lines in the warning colour, darktable's orange, and a drag's step undone, ending the drag, as darktable's; the operation refuses to prepare. Over the smoothing range the f64 fit never fails. Where the module's code panics instead, the graph draws no curve and a drag moves nothing.
- **The keys:** one focus stop, as the curve's, with the keys [above](#the-bands-graph). darktable's graph takes no keys.
- **What a screen reader hears of the warnings:** the graph's description says where the mask overflows the bands ("The mask's shadows go past the −8 EV band."), before the keys.
- **The reset:** a double-click, darktable's, resets the nine bands, each with `ResetControl`, which also releases their marks.
