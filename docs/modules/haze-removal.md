# Haze removal

Module `hazeremoval`, version 2. It works on layers, has no Auto and is switched (decision 32).

It is darktable's haze removal in the first pass (decision 2). darktable's page has no tabs, sections or presets, only two sliders, here in `gui_init`'s order under darktable's labels: Strength and Distance (decision 29).

It drives one operation, `hazeremoval` ([../ports/hazeremoval.md](../ports/hazeremoval.md)), which an unedited photo does not run. The operation is planned while the module is on. The module owns both of the operation's parameters.

darktable's two flags, `compatibility_mode` and `adaptive`, which `gui_changed` sets off and on whenever a slider moves, have no widget and are no parameter of the port. It renders as a new instance does, compatibility mode off and adaptive windows on (rule 12 of [README.md](README.md); the port's Parameters).

**Status:** built, in `crates/modules/src/haze_removal.rs`. The app draws it in the Correct tab from its definition (`docs/gui.md`, Module panels from definitions), alone there so far.

- In [README.md](README.md)'s order, darktable's (decision 38), it follows Sharpen and Retouch and comes before Chromatic aberrations, Lens correction and Denoise (profiled), whose modules are not built yet.
- Its rows are tests in `crates/modules/tests/haze_removal.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Switched** (decision 32; [README.md](README.md), rule 4). Haze removal is off on a new photo, as darktable's is, and renders nothing. Any write of a control turns it on, whatever the value, with darktable's defaults for the other, until its reset turns it off; resetting one control leaves it on. So the binding sets each control off its default, and on at its defaults the operation runs at its own defaults, a strength and a distance of 0.2, darktable's new instance.

- **Version 2.** Version 1 started the strength at 0 and planned the operation only while it was off 0, writing it then. A version 1 edit migrates on where its strength was off 0, its values as they were; its values are otherwise dropped, since they rendered nothing (row Z8).

**The viewport** estimates from the full resolution (decision 15), with no change to this module's controls or binding. At test 0166's strength of 0.9 it is 0.0052 to 0.0540 mean ΔE2000 from the export downscaled, from 2400×1600 to an eighth; estimating at the viewport's own scale gives 1.33 to 5.0 (the port's Neighbourhood and scale). A tiled viewport, as on lavapipe above 8.4 MP, still estimates at its own scale.

## Controls

Each control is one `hazeremoval` parameter as it is: darktable's sliders show their parameters with no factor or offset. Each range is darktable's hard range, the params' `$MIN` and `$MAX`; darktable sets no soft range, so each track spans its range. Each step is darktable's, what `dt_bauhaus_slider_get_step` gives, since these sliders set none ([Sliders against darktable](#sliders-against-darktable)). Control ids are darktable's parameter names.

| Control | Parameter | Range | Soft range: the track's span | Step | Decimals | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Strength (`strength`) | `strength` | −1 to 1 | the range | 0.01 | 2 | 0.2, darktable's |
| Distance (`distance`) | `distance` | 0 to 1 | the range | 0.01 | 3 | 0.2, darktable's |

The strength says how much haze to remove, a negative strength adding haze: the transmission is one less the strength times the haze the dark channel finds.

The distance says how far into the scene: the transmission never falls below t_min = exp(−distance × depth), so a smaller distance leaves the far scene alone.

The promises, the tooltips, are darktable's in British English: "Amount of haze reduction." and "Limit haze removal up to a specific spatial depth." A script sends `SetControl { module: "hazeremoval", control: "strength", value: 0.4, done: true }`, and `apertrust-cli edit --set hazeremoval.strength=0.4` does the same.

**Off until a control is written** (rule 4 of [README.md](README.md)). Both controls start at darktable's 0.2, the operation's own, so the binding writes each only while it is off it. A strength of 0, once on, changes each value by one rounding at most (the port's Deliberate differences), as darktable's does.

**Presets:** darktable defines none for this module.

### Sliders against darktable

Audited against darktable's `src/iop/hazeremoval.c`, its params and `gui_init` ([README.md](README.md), Sliders against darktable). `its_sliders_are_darktable_s`, in `crates/modules/src/haze_removal.rs`, holds the definition to this table.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Strength | −1 to 1; none | as it is | 2, `dt_bauhaus_slider_from_params`' | 0.01 | No |
| Distance | 0 to 1; none | as it is | 3, `dt_bauhaus_slider_set_digits(g->distance, 3)` | 0.01 | No |

Each step is a hundredth of a range whose top, the smaller of its width and its larger end's size, is 1. No slider differs, and both defaults are darktable's.

## Invariants

A switched module's rows (decision 32; [README.md](README.md), Rows every table has). Z6, which held the distance to nothing while the strength was 0, is withdrawn: on, it acts at any strength. No control's unit is Apertrust's: each is its parameter as darktable's slider shows it, and Z4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| Z0 | None: the module off, untouched or with every control kept | | The plan's instances and cache keys; the frame, on one pipeline that haze removal joins and leaves on the way | Instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| Z1 | The first step | The module on at darktable's defaults, a strength and a distance of 0.2: a control written at its default | Frame, ΔE2000 against no edit | darktable's behaviour, darktable's new instance removing haze | Measured and reported, without a bound (decision 22) | Ramp, sweep; the chart; every corpus raw |
| Z2 | Each control, moved and reset | Strength +0.5, −0.3, and 0.2, a write at its default; Distance 0.8 alone; Strength 0.4 with Distance 0.8; both at test 0166's 0.9 and 0.8; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan; the frame | Each move changes the frame; reset control by control, the module stays on and renders as Z1's; the module's reset turns it off and renders as no edit, haze removal leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| Z3 | Two controls, in either order | Strength 0.6 and Distance 0.5; Strength −0.4 and Distance 0.1 | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| Z4 | Every control, the darktable match | The values of darktable's tests 0026 and 0166, the golden cases `hazeremoval` and `hazeremoval-v2`, each field as it is | The parameters the module binds, and the passes the operation's `prepare` gives for them | darktable's parameters as the golden harness reads them, and so the passes it checks against darktable's render | Exact: every parameter bit for bit, every pass's uniform bytes the same | The cases' parameters, on the CPU |
| Z5 | Each control on a layer, Phase 4 | Z2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As Z2 | Sweep |
| Z7 | On at darktable's defaults | Every control's default made explicit; the module on at its defaults, and off | The parameters bound (`check::switched_defaults`); the plan | Its defaults bind the operation's own exactly; on at its defaults haze removal is planned with no parameter set, darktable's new instance; off, it is not planned | Exact | The CPU, a colour raw's context and an image that is not a raw |
| Z8 | Version 1's edits | Version 1 entries read from `.apxmp`: Strength 0.9 with Distance 0.8; Strength −0.35; Strength 0.2; and Distance 0.8 alone, a strength of 0, and nothing | The module's state; the operation's parameters, resolved over its schema | Those with the strength off 0 read on and bind what version 1 bound; the others read off, their values dropped, haze removal not planned | Exact | The CPU |

Z4 is how the module matches darktable. The golden cases check the operation against darktable-cli at the cases' settings (`docs/ports/hazeremoval.md`):

- a mean ΔE2000 of 0.0213 on test 0026, whose version 1 params run darktable's compatibility mode, which the port does not reproduce;
- 0.0025 on test 0166.

Z4 holds the module's controls set to those settings to bind those parameters. On test 0026 the distance, 0.2, is the module's default, so the binding leaves it to the operation's default, the same 0.2.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/haze_removal.rs`. The synthetic rows and Z4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS` and the chart's with `APERTRUST_TEST_IMAGES`, both `#[ignore]`d.

| Row | Result |
| --- | --- |
| Z0 | Holds, exactly. Frames byte-identical to no edit, the module off and untouched or with every control kept, on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that haze removal joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| Z1 | Measured (below) |
| Z2 | Holds, exactly, on the sweep and on the small raw through the viewport, and on the 90D through the viewport and the export: every move changes the frame, a write at the default among them; reset control by control, the module renders as on at darktable's defaults; the module's reset renders as no edit |
| Z3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| Z4 | Holds, bit for bit. Test 0026 binds a strength of 0.2 and leaves the distance, 0.2, to the operation's default; test 0166 binds 0.9 and 0.8. Each case's 25 passes from `prepare` hold the same uniform bytes as the harness's plan of the case |
| Z7 | Holds, exactly |
| Z8 | Holds, exactly |

Z1's ΔE2000 against no edit: darktable's new instance, a strength and a distance of 0.2. On the ramp, whose greys hold no haze, a mean of 8.3e-5, max 3.4e-4, 247 of 322 pixels not byte-identical; on the sweep a mean of 1.04, max 8.81, 45.1 % of pixels over 1.0. On the 31 corpus raws through the export, means at most 3.02, the largest 17.3, at most 98.3 % of a raw's pixels over 1.0.

The golden cases, with the module: `hazeremoval` a mean ΔE2000 of 0.0213, p95 0.0506, max 0.1319, and `hazeremoval-v2` 0.0025, p95 0.0062, max 0.1291, as the port measured them (Apple M4).

## In the panel

- **No tabs and no group labels:** darktable's two sliders, one under the other, in its order.
- **Tracks** are plain, filled from the default: the strength's from 0.2, the distance's from 0.2.
- **Values** show darktable's decimals, the strength two and the distance three, in the language's number format: "+0.40", "−0.30", "0.650". The strength is signed, as darktable's is, since its range spans zero, and a zero strength reads "+0.00".
- **Keyboard:** every row is a `SliderRow`, stepping by darktable's 0.01, Shift ten steps and Alt a tenth, never less than the last decimal shown, 0.01 for the strength and 0.001 for the distance, as darktable's `min_visible`. The value is rounded to the decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The viewport at high strengths** (above; decision 15): built, but in a tiled viewport, as on lavapipe above 8.4 MP, which still estimates at its own scale.
2. **The module's tooltip,** darktable's "remove fog and atmospheric hazing from images": a definition implies no message for the module's own tooltip.
3. **darktable's warning** where the ambient light cannot be estimated, "haze removal could not calculate ambient light due to image content": the operation guards a black A0 (the port's Deliberate differences, 4), and no message reaches the panel.

## Design choices

- **Control ids:** darktable's parameter names, `strength` and `distance`, so `--set` and scripts name what darktable's history names, as RGB primaries' do. They are darktable's labels too.
- **When the operation is planned:** while the module is on (decision 32), from the first write of a control until the module's reset, whatever the controls hold.
- **What binds:** each control only while off its default, darktable's 0.2, the operation's own.
- **The distance at a strength of 0:** enabled, as darktable draws it (no `gtk_widget_set_sensitive`). `Module::available` is kept for what darktable's own GUI disables (decision 30).
- **Units and decimals:** darktable's: the parameters as they are, the strength's two decimals and the distance's three. darktable's slider shows no percentage.
- **Row ids:** Z, for haZe removal, D left for the detail modules (denoise, diffuse or sharpen).
- **The files' names:** by the module's English name, `haze_removal.rs` and this page, `haze-removal.md`, apart from the operation's `hazeremoval` (the `tool-bindings` skill).
