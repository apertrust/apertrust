# Grain

Module `grain`, version 1. It works on the background only and has no Auto.

It is darktable's grain in the first pass (decision 2). darktable's page has no tabs, sections or presets: its three sliders are in `gui_init`'s order under darktable's labels in British spelling (decision 29), Coarseness, Strength and Mid-tones bias, as darktable's panel shows them: "1600 ISO", "25.00%" and "100.00%" at darktable's defaults.

It drives one operation, `grain` ([../ports/grain.md](../ports/grain.md)), which an unedited photo does not run. It is planned only while Strength is off 0, whatever Coarseness and Mid-tones bias hold (the port's Tool binding notes). The module owns all three of the operation's parameters. darktable's `channel`, which its GUI does not show and its `process()` ignores, is not ported ([README.md](README.md), Effect).

**Status:** built, in `crates/modules/src/grain.rs`. The app draws it in the Effect tab after Soften, from its definition (`docs/gui.md`, Module panels from definitions). In [README.md](README.md)'s order (decision 38) it follows Vignetting and Soften and comes before Bloom and Diffuse or sharpen, whose modules are not built yet.

- Its rows are tests in `crates/modules/tests/grain.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Planned: Grain becomes switched** (decision 32; [README.md](README.md), rule 4). It will be off until a control changes, then on with darktable's defaults for every control not set, until its reset turns it off. Until then the rest of this page describes the module as built. The change:

- The strength starts at darktable's 25 %, so its default in the table below changes, and the binding sets each control off its default, its gate on the strength gone.
- The module becomes version 2. A version 1 edit migrates on where its strength was off 0; its values are otherwise dropped, since they rendered nothing.
- Its rows become a switched module's ([README.md](README.md), Rows every table has): G0 becomes off and G1 the module on at 25 %; G2's resets follow the switched rows; G5 is withdrawn; new rows after G5 hold on at darktable's defaults and version 1's edits.
- Gaps 1 and 3 close with it.

## Controls

Each control is one `grain` parameter, shown as darktable's slider shows it: the coarseness as ISO, `scale` times darktable's `GRAIN_SCALE_FACTOR`, 213.2, with no decimals; the strength and the mid-tones bias as the percentages they are, with two.

The binding goes back as darktable's slider does, `value / factor` in f32 with darktable's f32 factor. So 20, 1600 and 6400 ISO bind darktable's minimum, default and maximum bit for bit, a whole ISO binds the quotient darktable's slider computes for it, and the golden cases' values bind their fields (row G4).

Each range is darktable's hard range in the unit shown, and each track spans it, as darktable sets no soft range. Each step is darktable's, what `dt_bauhaus_slider_get_step` gives over the range, since these sliders set none ([Sliders against darktable](#sliders-against-darktable)). Control ids are darktable's parameter names.

| Control | Parameter | Shown as | Range | Track's span | Step | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Coarseness (`scale`) | `scale` | ISO, 213.2 × `scale` | 20 to 6400 ISO (0.0938 to 30.02) | the range | 50 ISO | 1600 ISO (7.505), darktable's |
| Strength (`strength`) | `strength` | % | 0 % to 100 % | the range | 1 % | 0 %, untouched; darktable's new instance has 25 % |
| Mid-tones bias (`midtones_bias`) | `midtones_bias` | % | 0 % to 100 % | the range | 1 % | 100 %, darktable's |

The coarseness sets the grain's size, as a film's speed; the strength how much it moves lightness; the mid-tones bias how much the paper response makes it fall off towards black and white. The promises, the tooltips, are darktable's in British English: "The grain size (~ISO of the film).", "The strength of applied grain." and darktable's sentence on the paper response for the bias.

A script sends `SetControl { module: "grain", control: "strength", value: 25, done: true }`, and `apertrust-cli edit --set grain.strength=25` does the same.

**Strength 0 is untouched** (rule 4 of [README.md](README.md)). At 0 darktable's grain still moves lightness, by up to 1.2e-3 L at a bias of 100 (`docs/ports/grain.md`, Precision), so the binding sets nothing there and the operation stays out of the plan. The strength therefore starts at 0, where darktable's new instance has 25 %. The coarseness and the bias start at darktable's defaults, which are the operation's own, so the binding sets each only while it is off them.

**Coarseness and Mid-tones bias change nothing while Strength is 0** (row G5) and stay enabled there: darktable's grain has no `gui_changed`, so it never makes them insensitive.

**Presets:** darktable's `grain.c` has none. darktable's default strength, 25 %, is planned as the "darktable's default" preset ([README.md](README.md), rule 4) once the header's presets menu is built ([Gaps](#gaps)).

### Sliders against darktable

Audited against darktable's `src/iop/grain.c`, its params and `gui_init`, and against darktable's panel as it draws them ([README.md](README.md), Sliders against darktable). darktable's side is in its parameters' units, with the factor its slider shows them by; Apertrust's is the table above, in the units shown.

`its_sliders_are_darktable_s`, in `crates/modules/src/grain.rs`, holds the definition to this table, and the binding's factor to darktable's.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Coarseness | 20/213.2 to 6400/213.2; none | " ISO", × 213.2 (`dt_bauhaus_slider_set_factor`) | 0 (`dt_bauhaus_slider_set_digits`) | 50 ISO | No |
| Strength | 0 to 100; none | "%", as it is: its hard maximum lies beyond ±10, so `dt_bauhaus_slider_set_format` adds no factor | 2 | 1 % | No |
| Mid-tones bias | 0 to 100; none | as Strength's | 2 | 1 % | No |

The coarseness's step: its range, 29.9 in `scale`, is under 100, so darktable takes a hundredth of it as shown, 63.8 ISO, to the power of ten below it, 10, times 5, as 63.8 lies more than half a decade above: 50 ISO. The strength's and the bias's ranges reach 100, so each steps by 1.

The one difference from darktable's new instance is the strength's default, 0 rather than 25 %, by rule 4; no slider differs.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the layer row, since the module works on the background only, and G5 for the controls that act only with the strength.

No control's unit is Apertrust's: ISO and the percentages are darktable's own display of its parameters, and G4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| G0 | None: every control at its default, written at it or kept there | | `bind`; the plan's instances and cache keys; the frame, on one pipeline that grain joins and leaves on the way | `bind` returns nothing; instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| G1 | The first step | `grain` planned at the module's defaults made explicit: a strength of 0, 1600 ISO, 100 % | Frame, ΔE2000 against G0's | darktable's behaviour: at a strength of 0 the paper response's lookup still moves lightness, by up to 1.2e-3 L, and the round trip through Lab adds f32 rounding (`docs/ports/grain.md`, Precision); what leaving grain out at 0 hides, since the binding never plans it there (G5) | Measured and reported, without a bound (decision 22) | Ramp, sweep; every corpus raw |
| G2 | Each control, moved and reset | Strength 1 %, its step, and 25 %; with Strength at 25 %, Coarseness 1650 ISO and Mid-tones bias 99 %, each one step; all three at the `grain-agx` case's 3200 ISO, 71.3 % and 83 %; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan's operations and cache keys; the frame | Each move changes the frame; each reset as G0, grain leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| G3 | Two controls, in either order | Strength 25 % and Coarseness 800 ISO; Mid-tones bias 52.7 % and Strength 37.3 % | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| G4 | Every control, the darktable match | The values of darktable's five golden grain cases, test 0059's `grain`, `grain-fine`, `grain-channel` and `grain-v1`, and `grain-agx`, as its sliders show them: Coarseness the field × 213.2 in f32, unrounded, and a version 1 case's Mid-tones bias 0, as darktable's `legacy_params` gives it; then Coarseness rounded to the whole ISO the panel shows | The parameters the module binds, and the kernel's inputs through the operation's `prepare` | darktable's parameters bit for bit, and the kernel's inputs the golden harness's (below) | Exact, and one f32 step for `grain`'s rounded ISO | The cases' parameters, on the CPU |
| G5 | Coarseness and Mid-tones bias, while Strength is 0 | Coarseness 1650 ISO, one step, 20 and 6400 ISO; Mid-tones bias 99 %, one step, and 0 %; 3200 ISO with 83 % | `bind`; the plan's operations and cache keys; the frame | `bind` returns nothing, so grain stays out of the plan and the frame is no edit's | Exact | Sweep |

G4 must hold:

- **The parameters:** darktable's, bit for bit.
- **The kernel's inputs:** every word the golden harness checks against darktable's render, but the three offset terms, which are the provisional offset's (`docs/ports/grain.md`, Deliberate differences), and the gradient slots, which only the derivative entry points read.
- **The rounded ISO** binds the field exactly for four cases, and within one f32 step for `grain`. Its field, 0x41f0266c, one step under darktable's maximum, is reached only by an unrounded value, from the CLI or a script, where 6400 ISO binds the maximum.

G4 is how the module matches darktable. The golden cases check the operation against darktable-cli at the cases' settings and darktable's offset, a ΔE2000 mean of 0.0008 at most, max 0.095 (`docs/ports/grain.md`, Results). G4 holds the module's controls set to those settings to bind those parameters.

darktable's offset, a hash of the file name, is no field of darktable's params, so no module value reaches it. The engine renders every photo at the provisional offset 0 until the seed stored in the edit, planned (`docs/ports/grain.md`, Deliberate differences).

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/grain.rs`. The synthetic rows and G4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| G0 | Holds, exactly. `bind` sets nothing with every control written at its default, or kept there. Frames byte-identical to no edit on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that grain joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| G1 | ΔE2000 means of 0.0002 to 0.0008, max 0.001, none over 1.0 (below) |
| G2 | Holds, exactly, on the sweep and on the small raw through the viewport, at a scale of about 0.48, where the lattice averages the noise, and on the 90D through the viewport and the export. Every move changes the frame, each control's one step among them, and resets render as no edit |
| G3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| G4 | Holds, exactly (below) |
| G5 | Holds, exactly: grain stays out of the plan, and the frame is no edit's |

- **G1:** the ramp, a mean of 0.0003, max 0.001. The sweep, a mean of 0.0004, max 0.001, none over 1.0. The 31 corpus raws, means of 0.0002 (Fujifilm X-E4) to 0.0008 (Canon EOS 7D Mark II), the largest 0.001 on every raw, none over 1.0. So the first step moves lightness by the lookup's offset at a strength of 0, far below what any step of the strength adds.
- **G4:** all three parameters of all five cases bind darktable's fields bit for bit, `grain`'s 6399.9995 ISO among them, one f32 step under darktable's maximum. Every word of the kernel's inputs is the golden harness's but the offset terms, the provisional offset's, and the gradient slots. Rounded to the ISO shown, 100, 2300, 1150 and 3200 ISO bind their cases' fields exactly, and `grain`'s 6400 ISO binds darktable's maximum, one step above its field.

## In the panel

- **No tabs and no group labels:** darktable's three sliders, one under another, in its order.
- **Values** show with darktable's digits and their unit, as darktable writes them, the coarseness with no decimals and the others with two: "1600 ISO", "25.00%" and "100.00%", in the language's number format. Screen readers hear the same; the field a click opens holds the number alone (decision 42).
- **Tracks** are plain, over the whole range, filled from the default: the strength's from 0, the bias's from its end at 100 %.
- **Keyboard:** every row is a `SliderRow`, stepping by darktable's step, 50 ISO or 1 %, Shift ten steps and Alt a tenth, never less than the last decimal shown, 1 ISO or 0.01 %, as darktable's `min_visible`. The value is rounded to the decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **darktable's default as a preset.** Its 25 % strength, with the defaults' 1600 ISO and 100 %, waits for the header's presets menu, which no module has yet.
2. **The module's tooltip,** darktable's "simulate silver grains from film": a definition implies no message for the module's own tooltip.
3. **What a move at a strength of 0 shows.** In darktable a slider moved on a module that is off turns it on, at its 25 %, so the grain appears; here Coarseness and Mid-tones bias change nothing until the strength moves (G5), since grain stays out of the plan at 0.

## Design choices

- **Control ids:** darktable's parameter names, `scale` for the coarseness among them, so `--set` and scripts name what darktable's history names, as RGB primaries' and Colour balance RGB's do.
- **The coarseness's unit:** stored in ISO, as shown, the binding dividing by darktable's f32 factor in f32, as darktable's slider does.
  - A drag rounds to the ISO shown as `roundf(base · value) / base`, `base` the f32 213.2 (`_slider_set_normalized`). A typed value takes the same rounding, `dt_bauhaus_slider_set_val` setting `value / factor` through `_slider_set_normalized` too, both in f32.
  - With no digits a whole ISO rounds to itself, so the binding's `value / factor` is the quotient darktable's slider stores for it, before its round trip through the track's position, and every golden case's value round-trips bit for bit.
  - Dividing in f64 would bind 1129 of the 6381 whole ISO values from 20 to 6400 one f32 step from darktable's, and `grain`'s unrounded 6399.9995 ISO one step below its field (0x41f0266b for 0x41f0266c).
- **A unit for ISO:** `Unit::Iso`, `unit-iso` and `unit-iso-value`, "ISO" in every language, as darktable's " ISO" format is no translated string. Screen readers hear the unit.
- **What binds while the strength is on:** the strength, and each other control only while off its default, which is the operation's own, bit for bit.
- **Ranges and tracks:** darktable's hard ranges in the units shown, the tracks over them, as darktable sets no soft range.
- **Steps and decimals:** darktable's: its computed steps, 50 ISO and 1 %, and its digits, none and two.
- **Words:** darktable's labels and tooltips from its `en@truecase` catalogue, with British spelling ("modelling", "fall-off") and a full stop; Ukrainian and Polish from its `uk.po` and `pl.po` at the tag, capitalised as the other modules' are.
- **Row ids:** G, for Grain.
- **Row G5:** that the coarseness and the bias change nothing while the strength is 0, measured exactly.
- **The layer row:** left out, since the module works on the background only.
