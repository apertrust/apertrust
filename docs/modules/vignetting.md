# Vignetting

Module `vignette`, version 2. It works on layers, has no Auto and is switched (decision 32).

It is darktable's vignetting in the first pass (decision 2). darktable's page has no tabs, collapsible sections or presets of its own but darktable's lomo. Its controls are in `gui_init`'s order under darktable's labels in British spelling, with the position / form label before the centres (decision 29).

It drives one operation, `vignette` ([../ports/vignette.md](../ports/vignette.md), ported without dithering), which an unedited photo does not run. It is planned while the module is on. The module owns all eleven of the operation's parameters; `unbound`, which darktable shows no widget for, stays at the operation's default, on, darktable's.

**Status:** built, in `crates/modules/src/vignetting.rs`. The app draws it in the Effect tab, first, before Soften and Grain, from its definition (`docs/gui.md`, Module panels from definitions), as [README.md](README.md) orders the tab (decision 38).

- Its rows are tests in `crates/modules/tests/vignetting.rs`, and the panel's in `crates/app/src/modules.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Switched** (decision 32; [README.md](README.md), rule 4). Vignetting is off on a new photo, as darktable's is, and renders nothing. Any write of a control turns it on, whatever the value, with darktable's defaults for the others, until its reset turns it off; resetting one control leaves it on. So the binding sets each control off its default, and on at its defaults the operation runs at its own defaults, brightness and saturation −0.5 among them, darktable's new instance.

- **Version 2.** Version 1 started brightness and saturation at 0 and planned the vignette only while either was off 0, setting both. A version 1 edit migrates on where either was off 0, its values as they were and the other of the two written at 0, its version 1 default; its values are otherwise dropped, since they rendered nothing (row V8).

## Controls

Each control is one `vignette` parameter, shown as darktable's slider shows it, as it is ([Sliders against darktable](#sliders-against-darktable)):

- darktable's `dt_bauhaus_slider_set_format(…, "%")` on the radii adds a percent sign and no factor, their ranges reaching 200.
- Each range is darktable's hard range.
- Each step is darktable's, what `dt_bauhaus_slider_get_step` gives over that range, since `gui_init` sets no soft range.
- Each row's decimals are darktable's digits.

Control ids are the operation's parameter names, darktable's but for the centre's, `center.x` and `center.y` in darktable's params.

| Control | Parameter | Shown as | Range | Step | Decimals | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Fall-off start (`scale`) | `scale` | % | 0 % to 200 % | 1 % | 2 | 80 % |
| Fall-off radius (`falloff_scale`) | `falloff_scale` | % | 0 % to 200 % | 1 % | 2 | 50 % |
| Brightness (`brightness`) | `brightness` | a number | −1 to 1 | 0.01 | 3 | −0.5 |
| Saturation (`saturation`) | `saturation` | a number | −1 to 1 | 0.01 | 3 | −0.5 |
| *Position / form*, a group label | | | | | | |
| Horizontal centre (`center_x`) | `center_x` | a number, half widths from the middle, right positive | −1 to 1 | 0.01 | 3 | 0 |
| Vertical centre (`center_y`) | `center_y` | a number, half heights from the middle, down positive | −1 to 1 | 0.01 | 3 | 0 |
| Shape (`shape`) | `shape` | a number: 0 a rectangle, 1 an ellipse, 2 a diamond, above a star | 0 to 5 | 0.05 | 2 | 1 |
| Automatic ratio (`autoratio`) | `autoratio` | a toggle | off or on | | | off |
| Width/height ratio (`whratio`) | `whratio` | a number: below 1 the width over the height, above 1 two less the height over the width | 0 to 2 | 0.01 | 3 | 1 |
| Dithering (`dithering`) | `dithering` | a list: Off, 8-bit output and 16-bit output, the two noises not ported | | | | Off |

**Off until a control is written** ([README.md](README.md), rule 4). Every control starts at darktable's default, the operation's own, so the binding sets each only while it is off it. At brightness and saturation 0, once on, the vignette gives every pixel back, its values exactly, `col · (1 + w · 0)` and `col − (mv − col) · 0`, unclipped, but for a −0.0, which the sum turns to +0.0.

**The width/height ratio is unavailable while automatic ratio is on** (decision 30), as darktable's `gui_changed` makes it insensitive, `gtk_widget_set_sensitive(g->whratio, !p->autoratio)`. Automatic ratio takes the vignette's ratio from the image's, so the control can change nothing ([V6](#invariants)). It is drawn disabled and keeps its value, which acts again once automatic ratio is off. The session reports it after every change, and the panel follows a pending change at once through the module's `available`.

**Dithering** lists darktable's three options. Off, the operation's, is the one it takes. 8-bit and 16-bit output are not ported (`docs/ports/vignette.md`, Dithering), listed disabled and refused, so the row shows Off as darktable's untouched module does.

The promises, the tooltips, are darktable's in British English: "The radii scale of vignette for start of fall-off", "Strength of effect on brightness", "Horizontal offset of centre of the effect", "Shape factor: 0 produces a rectangle, 1 a circle or ellipse, 2 a diamond", "Enable to have the ratio automatically follow the image size", and the like.

A script sends `SetControl { module: "vignette", control: "brightness", value: -0.5, done: true }`, and `apertrust-cli edit --set vignette.brightness=-0.5` does the same.

**Presets:** darktable's lomo (fall-off from 40 % over 100 %, brightness −1, saturation +0.5, the ratio fixed at 1, shape 1), planned with the header's presets menu.

### Sliders against darktable

Audited against darktable's `src/iop/vignette.c`, its params and `gui_init`, and against its panel as darktable draws it: fall-off start 80.00%, fall-off radius 50.00%, brightness and saturation −0.500, then position / form, the centres +0.000, shape 1.00, the automatic ratio toggle, width/height ratio 1.000, dithering off ([README.md](README.md), Sliders against darktable).

`its_sliders_are_darktable_s`, in `crates/modules/src/vignetting.rs`, holds the definition to this table.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Fall-off start, fall-off radius | 0 to 200; none | %, × 1 | 2, from params | 1 % | No |
| Brightness, saturation | −1 to 1; none | × 1 | 3, `gui_init` | 0.01 | No |
| Horizontal and vertical centre | −1 to 1; none | × 1 | 3, `gui_init` | 0.01 | No |
| Shape | 0 to 5; none | × 1 | 2, from params | 0.05 | No |
| Width/height ratio | 0 to 2; none | × 1 | 3, `gui_init` | 0.01 | No |

## Invariants

A switched module's rows (decision 32; [README.md](README.md), Rows every table has), and V6 for the width/height ratio under automatic ratio. V0's clause for brightness and saturation at 0 with the other controls moved is withdrawn: on, those controls act whatever the strengths hold.

No control's unit is Apertrust's: every value is darktable's parameter as it is, and V4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| V0 | None: the module off, untouched or with every control kept | | The plan's instances and cache keys; the frame, on one pipeline that the vignette joins and leaves on the way | Instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| V1 | The first step | The module on at darktable's defaults, brightness and saturation −0.5: a control written at its default | Frame, ΔE2000 against no edit | darktable's behaviour, darktable's new instance darkening and desaturating towards the corners | Measured and reported, without a bound (decision 22) | Ramp, sweep, the small raw; every corpus raw |
| V2 | Each control, moved and reset | Brightness −0.5, a write at its default; fall-off start 40 % alone; saturation +0.3 with fall-off start 40 %; fall-off radius 100 % with brightness −1; the centre, the shape and a positive brightness; automatic ratio with saturation −0.43; every control at test 0045's values; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan; the frame | Each move changes the frame; reset control by control, the module stays on and renders as V1's; the module's reset turns it off and renders as no edit, the vignette leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| V3 | Two controls, in either order | Brightness −0.6 and fall-off start 40 %; saturation +0.3 and automatic ratio; brightness −0.81 and shape 3.7 | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| V4 | Every control, the darktable match | The values of the golden cases `vignette` (test 0045, dithering off), `vignette-autoratio`, `vignette-narrow`, `vignette-star` and `vignette-white`: darktable's fields as they are | The parameters the module binds, and the kernel's inputs through the operation's `prepare` | darktable's parameters, and the inputs the golden harness checks against darktable's render | Bit for bit, every parameter and every settings word; the gradient slots, which only the derivative entry points read and the harness chooses per case, apart | The cases' parameters, on the CPU |
| V5 | Each control on a layer, Phase 4 | V2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As V2 | Sweep |
| V6 | Width/height ratio, while automatic ratio is on | 0.5 and 1.6, with brightness −0.6 | Whether it is available (`Module::available`); the frame's pixels | Unavailable, and the same frame whatever it holds; with automatic ratio off, available, and the frames differ | Exact, the pixels | Sweep; the panel, on the CPU |
| V7 | On at darktable's defaults | Every control's default made explicit; the module on at its defaults, and off | The parameters bound (`check::switched_defaults`); the plan | Its defaults bind the vignette's own exactly, `unbound` on among them; on at its defaults the vignette is planned with no parameter set, darktable's new instance; off, it is not planned | Exact | The CPU, a colour raw's context and an image that is not a raw |
| V8 | Version 1's edits | Version 1 entries read from `.apxmp`: saturation +0.3 with fall-off start 40 %; brightness −0.5; test 0045's strengths and shape; and fall-off start 40 % with automatic ratio, both strengths at 0, and nothing | The module's state; the vignette's parameters, resolved over its schema | Those with a strength off 0 read on and bind what version 1 bound, the other strength 0 and `unbound` on; the others read off, their values dropped, the vignette not planned | Exact | The CPU |

V4 is how the module matches darktable. The golden cases check the operation against darktable-cli at each case's settings, a ΔE2000 mean of 0.0000 and a max of 0.0002 at most (`docs/ports/vignette.md`, Results). V4 holds the module's controls set to those settings to bind those parameters.

The sixth case, `vignette-clip`, clips (`unbound` off), which the module never does, so it is out of the module's reach; a test says so, and that the five are every other vignette case.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/vignetting.rs`. The synthetic rows and V4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| V0 | Holds, exactly. Frames byte-identical to no edit, the module off and untouched or with every control kept, on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that the vignette joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| V1 | Measured (below) |
| V2 | Holds, exactly, on the sweep and on the small raw through the viewport, and on the 90D through the viewport and the export: every move changes the frame, a write at the default among them; reset control by control, the module renders as on at darktable's defaults; the module's reset renders as no edit |
| V3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| V4 | Holds, bit for bit, on all five cases: every parameter the module binds is darktable's field, and the kernel's twelve settings words are the golden harness's |
| V6 | Holds: unavailable while automatic ratio is on, the same pixels at 0.5 and 1.6; available with it off, and they differ (below) |
| V7 | Holds, exactly |
| V8 | Holds, exactly |

**V1:** darktable's new instance, brightness and saturation −0.5: on the ramp a mean ΔE2000 of 0.111, max 2.26, 5.3 % of pixels over 1.0; on the sweep a mean of 0.604, max 12.2, 17.1 % over 1.0; on the small raw a mean of 1.31, max 13.5, 28.3 % over 1.0. On the 31 corpus raws through the export, means at most 1.45, the largest 20.4.

**V6 in the panel:** the panel draws the ratio disabled while the change is pending and once the session reports it, keeping 1.5, refusing a drag and Delete, and live again at once when automatic ratio goes off (`vignetting_s_ratio_is_drawn_disabled_while_automatic_ratio_is_on`).

## In the panel

- **No tabs and no collapsible groups:** darktable's controls one under another in its order, the group label Position / form, darktable's section label, after Saturation (`vignetting_draws_darktable_s_page_in_order`).
- **Values** show darktable's digits, the radii with their unit, in the language's number format: "80.00%", the shape "1.00", the ratio "1.000". A slider whose range spans zero is signed, as darktable's is, zero included: brightness and saturation "−0.500" at rest, the centres "+0.000".
- **Automatic ratio** is a toggle row, **Dithering** a list, its two noises listed disabled.
- **The width/height ratio** is drawn disabled while automatic ratio is on, its value kept.
- **Keyboard:** every slider is a `SliderRow`, stepping by darktable's step, Shift ten steps and Alt a tenth, never less than the last decimal shown. The value is rounded to the decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The vignette on the image.** darktable draws the vignette's outline over the image, with handles for its inner and outer radii and its centre that a drag moves, and Ctrl keeps the ratio (`gui_post_expose`, `mouse_moved`, `mouse_actions`). Apertrust's on-image tools come later, their points mapped by the engine (decision 16).
2. **Presets:** darktable's lomo waits for the header's presets menu.
3. **The fill,** across the renderer: darktable fills each track from 0, the fall-off start's from its left end to 80 %; Apertrust's rows fill from the control's default, so the radii and the shape show no fill at rest.
4. **Dithering's noises,** not ported, and with them the raised-cosine fall-off darktable's CPU path uses only with dithering (`docs/ports/vignette.md`, Dithering and Deliberate differences).
5. **The module's tooltip,** darktable's "simulate a lens fall-off close to edges": a definition implies no message for the module's own tooltip.

## Design choices

- **Control ids:** the operation's parameter names, `scale`, `falloff_scale`, `center_x` and `center_y` among them, so `--set` and scripts name what darktable's history names, as RGB primaries' do.
- **Switched** (decision 32): off until a control is written, every default darktable's, brightness and saturation −0.5 among them, and `unbound` at the operation's default, on. An untouched module never changes the image.
- **Dithering:** a list with Off live and the two noises not ported, as Exposure's Mode lists its automatic option, so the row shows what darktable's does. Option ids are `off`, `output_8bit` and `output_16bit`, since an id starts with a letter, each the operation's choice at the same place. The map from option to choice comes with the noises' port; the binding sets none until then.
- **Rows:** a switched module's, V0 to V4, V7 and V8, and V6. V4 covers the five cases the module can reach; `vignette-clip` needs `unbound` as a control, which darktable does not show.
- **The panel's tests:** in `crates/app/src/modules.rs`, beside every other module's, since the renderer is the app's and `apertrust-modules` depends on neither egui nor the app. The session's availability report is tested on `testing::Sample` (decision 30).
- **Row ids:** V, for Vignetting.
- **The files' names:** by the module's English name, `vignetting.rs` and this table, `vignetting.md`, apart from the operation's `vignette` (the `tool-bindings` skill).
