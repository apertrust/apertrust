# Grain

Module `grain`, version 2. It works on the background only, has no Auto and is switched (decision 32).

It is darktable's grain in the first pass (decision 2). darktable's page has no tabs, sections or presets: its three sliders are in `gui_init`'s order under darktable's labels in British spelling (decision 29), Coarseness, Strength and Mid-tones bias, as darktable's panel shows them: "1600 ISO", "25.00%" and "100.00%" at darktable's defaults.

It drives one operation, `grain` ([../ports/grain.md](../ports/grain.md)), which an unedited photo does not run. It is planned while the module is on. The module owns all three of the operation's parameters. darktable's `channel`, which its GUI does not show and its `process()` ignores, is not ported ([README.md](README.md), Effect).

**Status:** built, in `crates/modules/src/grain.rs`. The app draws it in the Effect tab after Soften, from its definition (`docs/gui.md`, Module panels from definitions). In [README.md](README.md)'s order (decision 38) it follows Vignetting and Soften and comes before Bloom and Diffuse or sharpen, whose modules are not built yet.

- Its rows are tests in `crates/modules/tests/grain.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Switched** (decision 32; [README.md](README.md), rule 4). Grain is off on a new photo, as darktable's is, and renders nothing. Any write of a control turns it on, whatever the value, with darktable's defaults for the others, until its reset turns it off; resetting one control leaves it on. So the binding sets each control off its default, and on at its defaults the operation runs at its own defaults, darktable's new instance.

- **Version 2.** Version 1 started the strength at 0 and planned grain only while it was off 0. A version 1 edit migrates on where its strength was off 0, its values as they were; its values are otherwise dropped, since they rendered nothing (row G7).

## Controls

Each control is one `grain` parameter, shown as darktable's slider shows it: the coarseness as ISO, `scale` times darktable's `GRAIN_SCALE_FACTOR`, 213.2, with no decimals; the strength and the mid-tones bias as the percentages they are, with two.

The binding goes back as darktable's slider does, `value / factor` in f32 with darktable's f32 factor. So 20, 1600 and 6400 ISO bind darktable's minimum, default and maximum bit for bit, a whole ISO binds the quotient darktable's slider computes for it, and the golden cases' values bind their fields (row G4).

Each range is darktable's hard range in the unit shown, and each track spans it, as darktable sets no soft range. Each step is darktable's, what `dt_bauhaus_slider_get_step` gives over the range, since these sliders set none ([Sliders against darktable](#sliders-against-darktable)). Control ids are darktable's parameter names.

| Control | Parameter | Shown as | Range | Track's span | Step | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Coarseness (`scale`) | `scale` | ISO, 213.2 × `scale` | 20 to 6400 ISO (0.0938 to 30.02) | the range | 50 ISO | 1600 ISO (7.505), darktable's |
| Strength (`strength`) | `strength` | % | 0 % to 100 % | the range | 1 % | 25 %, darktable's |
| Mid-tones bias (`midtones_bias`) | `midtones_bias` | % | 0 % to 100 % | the range | 1 % | 100 %, darktable's |

The coarseness sets the grain's size, as a film's speed; the strength how much it moves lightness; the mid-tones bias how much the paper response makes it fall off towards black and white. The promises, the tooltips, are darktable's in British English: "The grain size (~ISO of the film).", "The strength of applied grain." and darktable's sentence on the paper response for the bias.

A script sends `SetControl { module: "grain", control: "strength", value: 25, done: true }`, and `apertrust-cli edit --set grain.strength=25` does the same.

**Off until a control is written** (rule 4 of [README.md](README.md)). Every control starts at darktable's default, the operation's own, so the binding sets each only while it is off it. A strength of 0, once on, renders darktable's lookup, which still moves lightness by up to 1.2e-3 L at a bias of 100 (`docs/ports/grain.md`, Precision), as darktable's does.

**Presets:** darktable's `grain.c` has none.

### Sliders against darktable

Audited against darktable's `src/iop/grain.c`, its params and `gui_init`, and against darktable's panel as it draws them ([README.md](README.md), Sliders against darktable). darktable's side is in its parameters' units, with the factor its slider shows them by; Apertrust's is the table above, in the units shown.

`its_sliders_are_darktable_s`, in `crates/modules/src/grain.rs`, holds the definition to this table, and the binding's factor to darktable's.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Coarseness | 20/213.2 to 6400/213.2; none | " ISO", × 213.2 (`dt_bauhaus_slider_set_factor`) | 0 (`dt_bauhaus_slider_set_digits`) | 50 ISO | No |
| Strength | 0 to 100; none | "%", as it is: its hard maximum lies beyond ±10, so `dt_bauhaus_slider_set_format` adds no factor | 2 | 1 % | No |
| Mid-tones bias | 0 to 100; none | as Strength's | 2 | 1 % | No |

The coarseness's step: its range, 29.9 in `scale`, is under 100, so darktable takes a hundredth of it as shown, 63.8 ISO, to the power of ten below it, 10, times 5, as 63.8 lies more than half a decade above: 50 ISO. The strength's and the bias's ranges reach 100, so each steps by 1.

No slider differs, and every default is darktable's.

## Invariants

A switched module's rows (decision 32; [README.md](README.md), Rows every table has), but the layer row, since the module works on the background only. G5, which held the coarseness and the bias to nothing while the strength was 0, is withdrawn: on, they act at any strength.

No control's unit is Apertrust's: ISO and the percentages are darktable's own display of its parameters, and G4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| G0 | None: the module off, untouched or with every control kept | | The plan's instances and cache keys; the frame, on one pipeline that grain joins and leaves on the way | Instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| G1 | The first step | The module on at darktable's defaults, 1600 ISO, 25 % and 100 %: a control written at its default | Frame, ΔE2000 against no edit | darktable's behaviour, darktable's new instance | Measured and reported, without a bound (decision 22) | Ramp, sweep; every corpus raw |
| G2 | Each control, moved and reset | Strength 0, 26 % and 25 %, a write at its default; Coarseness 1650 ISO and Mid-tones bias 99 %, each one step; all three at the `grain-agx` case's 3200 ISO, 71.3 % and 83 %; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan's operations and cache keys; the frame | Each move changes the frame; reset control by control, the module stays on and renders as G1's; the module's reset turns it off and renders as no edit, grain leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| G3 | Two controls, in either order | Strength 25 % and Coarseness 800 ISO; Mid-tones bias 52.7 % and Strength 37.3 % | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| G4 | Every control, the darktable match | The values of darktable's five golden grain cases, test 0059's `grain`, `grain-fine`, `grain-channel` and `grain-v1`, and `grain-agx`, as its sliders show them: Coarseness the field × 213.2 in f32, unrounded, and a version 1 case's Mid-tones bias 0, as darktable's `legacy_params` gives it; then Coarseness rounded to the whole ISO the panel shows | The parameters the module binds, and the kernel's inputs through the operation's `prepare` | darktable's parameters bit for bit, and the kernel's inputs the golden harness's (below) | Exact, and one f32 step for `grain`'s rounded ISO | The cases' parameters, on the CPU |
| G6 | On at darktable's defaults | Every control's default made explicit; the module on at its defaults, and off | The parameters bound (`check::switched_defaults`); the plan | Its defaults bind grain's own exactly; on at its defaults grain is planned with no parameter set, darktable's new instance; off, it is not planned | Exact | The CPU, a colour raw's context and an image that is not a raw |
| G7 | Version 1's edits | Version 1 entries read from `.apxmp`: Strength 37.3 % and 800 ISO; 25 % and a bias of 52.7 %; 100 % and a bias of 0 %; and 3200 ISO alone, a strength of 0, and nothing | The module's state; grain's parameters, resolved over its schema | Those with the strength off 0 read on and bind what version 1 bound; the others read off, their values dropped, grain not planned | Exact | The CPU |

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
| G0 | Holds, exactly. Frames byte-identical to no edit, the module off and untouched or with every control kept, on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that grain joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| G1 | Measured (below) |
| G2 | Holds, exactly, on the sweep and on the small raw through the viewport, at a scale of about 0.48, where the lattice averages the noise, and on the 90D through the viewport and the export. Every move changes the frame, each control's one step and a write at the default among them; reset control by control, the module renders as on at darktable's defaults; the module's reset renders as no edit |
| G3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| G4 | Holds, exactly (below) |
| G6 | Holds, exactly |
| G7 | Holds, exactly |

- **G1:** darktable's new instance, grain at 25 %: on the ramp a mean ΔE2000 of 0.400, max 4.93, 13.0 % of pixels over 1.0; on the sweep a mean of 0.569, max 6.55, 18.8 % over 1.0. On the 31 corpus raws through the export, means at most 1.79, the largest 8.08, at most 66.7 % of a raw's pixels over 1.0.
- **G4:** all three parameters of all five cases bind darktable's fields bit for bit, `grain`'s 6399.9995 ISO among them, one f32 step under darktable's maximum. Every word of the kernel's inputs is the golden harness's but the offset terms, the provisional offset's, and the gradient slots. Rounded to the ISO shown, 100, 2300, 1150 and 3200 ISO bind their cases' fields exactly, and `grain`'s 6400 ISO binds darktable's maximum, one step above its field.

## In the panel

- **No tabs and no group labels:** darktable's three sliders, one under another, in its order.
- **Values** show with darktable's digits and their unit, as darktable writes them, the coarseness with no decimals and the others with two: "1600 ISO", "25.00%" and "100.00%", in the language's number format. Screen readers hear the same; the field a click opens holds the number alone (decision 42).
- **Tracks** are plain, over the whole range, filled from the default: the strength's from 25 %, the bias's from its end at 100 %.
- **Keyboard:** every row is a `SliderRow`, stepping by darktable's step, 50 ISO or 1 %, Shift ten steps and Alt a tenth, never less than the last decimal shown, 1 ISO or 0.01 %, as darktable's `min_visible`. The value is rounded to the decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The module's tooltip,** darktable's "simulate silver grains from film": a definition implies no message for the module's own tooltip.

## Design choices

- **Control ids:** darktable's parameter names, `scale` for the coarseness among them, so `--set` and scripts name what darktable's history names, as RGB primaries' and Colour balance RGB's do.
- **The coarseness's unit:** stored in ISO, as shown, the binding dividing by darktable's f32 factor in f32, as darktable's slider does.
  - A drag rounds to the ISO shown as `roundf(base · value) / base`, `base` the f32 213.2 (`_slider_set_normalized`). A typed value takes the same rounding, `dt_bauhaus_slider_set_val` setting `value / factor` through `_slider_set_normalized` too, both in f32.
  - With no digits a whole ISO rounds to itself, so the binding's `value / factor` is the quotient darktable's slider stores for it, before its round trip through the track's position, and every golden case's value round-trips bit for bit.
  - Dividing in f64 would bind 1129 of the 6381 whole ISO values from 20 to 6400 one f32 step from darktable's, and `grain`'s unrounded 6399.9995 ISO one step below its field (0x41f0266b for 0x41f0266c).
- **A unit for ISO:** `Unit::Iso`, `unit-iso` and `unit-iso-value`, "ISO" in every language, as darktable's " ISO" format is no translated string. Screen readers hear the unit.
- **What binds while the module is on:** each control only while off its default, which is the operation's own, bit for bit.
- **Ranges and tracks:** darktable's hard ranges in the units shown, the tracks over them, as darktable sets no soft range.
- **Steps and decimals:** darktable's: its computed steps, 50 ISO and 1 %, and its digits, none and two.
- **Words:** darktable's labels and tooltips from its `en@truecase` catalogue, with British spelling ("modelling", "fall-off") and a full stop; Ukrainian and Polish from its `uk.po` and `pl.po` at the tag, capitalised as the other modules' are.
- **Row ids:** G, for Grain.
- **The layer row:** left out, since the module works on the background only.
