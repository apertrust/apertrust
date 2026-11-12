# Soften

Module `soften`, version 2. It works on layers, has no Auto and is switched (decision 32).

It is darktable's soften in the first pass (decision 2). darktable's page has no tabs, sections or presets, only four sliders, here in `gui_init`'s order under darktable's labels in British spelling (decision 29): Size, Saturation, Brightness and Mix. darktable's panel shows them as "50.00%", "100.00%", "+0.33 EV" and "50.00%" at its defaults.

It drives one operation, `soften` ([../ports/soften.md](../ports/soften.md)), which an unedited photo does not run: it is planned while the module is on. The module owns all four of the operation's parameters; none is fixed ([README.md](README.md), Effect).

**Status:** built, in `crates/modules/src/soften.rs`. The app draws it in the Effect tab from its definition (`docs/gui.md`, Module panels from definitions), after Vignetting and before Grain.

- darktable's preset puts soften in no group, so it takes Effect, its group in darktable's "modules: all" ([README.md](README.md), decision 38).
- Its rows are tests in `crates/modules/tests/soften.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Switched** (decision 32; [README.md](README.md), rule 4). Soften is off on a new photo, as darktable's is, and renders nothing. Any write of a control turns it on, whatever the value, with darktable's defaults for the others, until its reset turns it off; resetting one control leaves it on. So the binding sets each control off its default, and on at its defaults the operation runs at its own defaults, darktable's new instance.

- **Version 2.** Version 1 started the mix at 0 and planned soften only while it was off 0. A version 1 edit migrates on where its mix was off 0, its values as they were; its values are otherwise dropped, since they rendered nothing (row S8).

## Controls

Each control is one `soften` parameter, shown as darktable's slider shows it, as it is, with two decimals: the size, the saturation and the mix as percentages, the brightness in EV. The binding passes each as it is.

Each range is darktable's hard range, and each track spans it, as darktable sets no soft range. Each step is darktable's, what `dt_bauhaus_slider_get_step` gives over the range, since these sliders set none ([Sliders against darktable](#sliders-against-darktable)). Control ids are darktable's parameter names, the mix's `amount`.

| Control | Parameter | Shown as | Range | Track's span | Step | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Size (`size`) | `size` | % | 0 % to 100 % | the range | 1 % | 50 %, darktable's |
| Saturation (`saturation`) | `saturation` | % | 0 % to 100 % | the range | 1 % | 100 %, darktable's |
| Brightness (`brightness`) | `brightness` | EV | −2 to +2 EV | the range | 0.01 EV | +0.33 EV, darktable's |
| Mix (`amount`) | `amount` | % | 0 % to 100 % | the range | 1 % | 50 %, darktable's |

Soften mixes a blurred copy of the photo into it, the Orton effect:

- the size sets the blur's radius, in percent of the largest radius, 1 % of the image's diagonal, in whole pixels;
- the saturation and the brightness scale the copy's HSL saturation and lightness before the blur;
- the mix is the copy's weight.

The promises, the tooltips, are darktable's in British English: "The size of blur.", "The saturation of blur.", "The brightness of blur." and "The mix of effect.". A script sends `SetControl { module: "soften", control: "amount", value: 50, done: true }`, and `apertrust-cli edit --set soften.amount=50` does the same.

**Off until a control is written** (rule 4 of [README.md](README.md)). Every control starts at darktable's default, the operation's own, so the binding sets each only while it is off it. A mix of 0, once on, returns the operation's input (`docs/ports/soften.md`, Tool binding notes).

**Presets:** darktable's `soften.c` has none.

### Sliders against darktable

Audited against darktable's `src/iop/soften.c`, its params and `gui_init`, and against darktable's panel as it draws them ([README.md](README.md), Sliders against darktable). darktable's side is in its parameters' units, which are the units shown; Apertrust's is the table above. `its_sliders_are_darktable_s`, in `crates/modules/src/soften.rs`, holds the definition to this table.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Size | 0 to 100; none | "%", as it is: its hard maximum lies beyond ±10, so `dt_bauhaus_slider_set_format` adds no factor | 2 | 1 % | No |
| Saturation | 0 to 100; none | as Size's | 2 | 1 % | No |
| Brightness | −2 to 2; none | " EV", as it is, signed, as its range crosses 0 | 2 | 0.01 EV | No |
| Mix | 0 to 100; none | as Size's | 2 | 1 % | No |

The brightness's step: its range's top, the smaller of its width, 4, and its larger end, 2, is under 100, so darktable takes a hundredth of it, 0.02 EV, to the power of ten below it, 0.01, which it lies less than half a decade above. The others reach 100 and step by 1. No slider differs, and every default is darktable's.

## Invariants

A switched module's rows (decision 32; [README.md](README.md), Rows every table has). S6, which held the other controls to nothing while the mix was 0, is withdrawn: on, they act at any mix. No control's unit is Apertrust's: the percentages and EV are darktable's parameters as they are, and S4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| S0 | None: the module off, untouched or with every control kept | | The plan's instances and cache keys; the frame, on one pipeline that soften joins and leaves on the way | Instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| S1 | The first step | The module on at darktable's defaults, 50 %, 100 %, +0.33 EV and a mix of 50 %: a control written at its default | Frame, ΔE2000 against no edit, and its values and bits | darktable's behaviour, darktable's new instance | Measured and reported, without a bound (decision 22) | Ramp, sweep; every corpus raw |
| S2 | Each control, moved and reset | Mix 51 %, its step, and 50 %, a write at its default; Size 100 %, Saturation 50 % and Brightness −1 EV; Size, Brightness and Mix at the golden case's 33 %, +0.5 EV and 75 %; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan's operations and cache keys; the frame | Each move changes the frame; reset control by control, the module stays on and renders as S1's; the module's reset turns it off and renders as no edit, soften leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| S3 | Two controls, in either order | Mix 50 % and Size 100 %; Brightness −0.5 EV and Mix 75 % | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| S4 | Every control, the darktable match | The values of darktable's test 0051-soften, the golden case `soften`, as its sliders show them: 33 %, 100 %, +0.5 EV, 75 % | The parameters the module binds | darktable's parameters bit for bit, which are the very values the golden harness plans the operation with against darktable's render | Exact | The case's parameters, on the CPU |
| S5 | Each control on a layer, Phase 4 | S2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As S2 | Sweep |
| S7 | On at darktable's defaults | Every control's default made explicit; the module on at its defaults, and off | The parameters bound (`check::switched_defaults`); the plan | Its defaults bind soften's own exactly; on at its defaults soften is planned with no parameter set, darktable's new instance; off, it is not planned | Exact | The CPU, a colour raw's context and an image that is not a raw |
| S8 | Version 1's edits | Version 1 entries read from `.apxmp`: Size 33 % with Mix 75 %; Mix 50 %; Brightness −2 EV with Mix 1 %; and Size 100 % alone, a mix of 0, and nothing | The module's state; soften's parameters, resolved over its schema | Those with the mix off 0 read on and bind what version 1 bound; the others read off, their values dropped, soften not planned | Exact | The CPU |

S4 is how the module matches darktable. The golden case checks the operation against darktable-cli at the case's settings, a ΔE2000 mean of 0.0001, max 0.0042 (`docs/ports/soften.md`, Results), as a one-instance plan of the engine with the harness's parameter values. S4 holds the module's controls set to those settings to bind those values, bit for bit.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/soften.rs`. The synthetic rows and S4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| S0 | Holds, exactly. Frames byte-identical to no edit, the module off and untouched or with every control kept, on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that soften joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| S1 | Measured: darktable's new instance, a mix of 50 %: on the ramp a mean ΔE2000 of 0.834, max 8.73, 31.1 % of pixels over 1.0; on the sweep a mean of 3.83, max 31.5, 76.6 % over 1.0. On the 31 corpus raws through the export, means at most 9.06, the largest 54.2, at most 99.2 % of a raw's pixels over 1.0, every raw exported within the default budget. |
| S2 | Holds, exactly, on the sweep and on the small raw through the viewport, and on the 90D through the viewport and the export. Every move changes the frame, a write at the default among them; reset control by control, the module renders as on at darktable's defaults; the module's reset renders as no edit |
| S3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| S4 | Holds, exactly: all four parameters bind darktable's fields bit for bit, and the harness's plan values |
| S5 | Waits for Phase 4's layers |
| S7 | Holds, exactly |
| S8 | Holds, exactly |

### Exports of the largest raws

The operation's two scratch images, 24 bytes a pixel of the whole image, are held across an export's tiles. Each tile is sized without that scratch (decision 15), so at the default budget of 2048 MiB (`engine.memory_budget`) the corpus's largest raws export with the mix on:

| Raw | Size | Tiles | Scratch held |
| --- | --- | --- | --- |
| Phase One IQ3 | 101 MP | 9 | 2314 MiB |
| Olympus E-M1 Mark III | 80.6 MP | 6 | 1845 MiB |
| Panasonic DC-S5M2 | 96 MP | 6 | 2197 MiB |

Each logs under `-d memory` its held scratch with its tiles over the budget (M4, whose binding of 4096 MiB is not the limit; `docs/ports/soften.md`, Results, Exports of the largest raws).

The viewport on the corpus meets decision 15's target: all 31 corpus raws at every scale (`docs/ports/soften.md`, Results, On the corpus).

## In the panel

- **No tabs and no group labels:** darktable's four sliders, one under another, in its order.
- **Values** show two decimals with their unit, as darktable writes them, in the language's number format: "50.00%", "+0.33 EV". The brightness is signed, as its range spans zero. Screen readers hear the same; the field a click opens holds the number alone (decision 42).
- **Tracks** are plain, over the whole range, filled from the default: the mix's from 50 %, the brightness's from +0.33 EV.
- **Keyboard:** every row is a `SliderRow`, stepping by darktable's step, 1 % or 0.01 EV, Shift ten steps and Alt a tenth, never less than 0.01, the last decimal shown, as darktable's `min_visible`. The value is rounded to the decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The module's tooltip,** darktable's "create a softened image using the Orton effect": a definition implies no message for the module's own tooltip.

## Design choices

- **Control ids:** darktable's parameter names, `amount` for the mix among them, so `--set` and scripts name what darktable's history names, as Grain's and RGB primaries' do.
- **Switched** (decision 32): off until a control is written, every default darktable's.
- **What binds while the module is on:** each control only while off its default, which is the operation's own, bit for bit.
- **Ranges, tracks, steps and decimals:** darktable's: its hard ranges, the tracks over them, as darktable sets no soft range, its computed steps, 1 % and 0.01 EV, and two decimals.
- **Words:** darktable's labels and tooltips from its `en@truecase` catalogue, with a full stop. Ukrainian and Polish from its `uk.po` and `pl.po` at the tag, capitalised as the other modules' are, Ukrainian's "Пом'якшення" with darktable's apostrophe.
- **Row ids:** S, for Soften.
- **S2's size:** moved to 100 %, not by one step: the radius moves in whole pixels, so a step changes nothing on the small test images.
