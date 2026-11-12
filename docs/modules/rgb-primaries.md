# RGB primaries

Module `primaries`, version 2. It works on layers, has no Auto and is switched (decision 32).

It is darktable's rgb primaries in the first pass (decision 2). darktable's page has no tabs, sections or presets, only eight sliders, here in `gui_init`'s order under darktable's labels in British spelling (decision 29).

It drives one operation, `primaries` ([../ports/primaries.md](../ports/primaries.md)), which an unedited photo does not run. The operation is planned while the module is on, as in darktable, where rgb primaries is off until the user turns it on (decision 32). The module owns all eight of the operation's parameters; none is fixed.

**Status:** built, in `crates/modules/src/rgb_primaries.rs`. The app draws it in the Colour tab from its definition (`docs/gui.md`, Module panels from definitions), after Colour balance RGB and before Colour calibration.

- In [README.md](README.md)'s order (decision 38) the colour equalizer, whose module is not built yet, comes between Colour balance RGB and it.
- Its rows are tests in `crates/modules/tests/rgb_primaries.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Switched** (decision 32; [README.md](README.md), rule 4). RGB primaries is off on a new photo, as darktable's rgb primaries is, and renders nothing. Any write of a control turns it on, whatever the value, with darktable's defaults for the others, until its reset turns it off; resetting one control leaves it on. Its defaults are darktable's, the operation's own, so its controls and binding are as they were: on at its defaults, the operation runs at its own defaults, darktable's new instance.

- **Version 2.** Version 1 planned the operation while a control was off its default. A version 1 edit migrates on where a value was off its defaults, its values as they were; its values are otherwise dropped, since they rendered nothing (row P8).

## Controls

Each control is one `primaries` parameter, shown as darktable's slider shows it, with darktable's factor and offset and its one decimal:

- a hue, in radians, as degrees, times 180/π as darktable's f32 `RAD_2_DEG` holds it;
- a primary's purity as 100 times the parameter less 100, so 1, a primary on the gamut's edge, is 0 %;
- the tint's purity as 100 times the parameter.

The binding goes back as darktable's slider does with a typed value, `(value − offset) / factor` in f32. Each range is darktable's hard range in the unit shown; 180° binds 3.1415925, one f32 step inside darktable's π, the same turn.

Each step is darktable's, what `dt_bauhaus_slider_get_step` gives over the soft range, since these sliders set none. The soft range is the span its track shows, as darktable's does ([Sliders against darktable](#sliders-against-darktable)). Control ids are darktable's parameter names.

| Control | Parameter | Shown as | Range | Soft range: the track's span | Step | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Red hue (`red_hue`) | `red_hue` | degrees | −180° to 180° | ±20° | 0.1° | 0° |
| Red purity (`red_purity`) | `red_purity` | %, 100 × purity − 100 | −99 % to +400 % (0.01 to 5) | −50 % to +50 % | 1 % | 0 % (1) |
| Green hue (`green_hue`) | `green_hue` | degrees | as Red hue's | ±20° | 0.1° | 0° |
| Green purity (`green_purity`) | `green_purity` | as Red purity's | as Red purity's | −50 % to +50 % | 1 % | 0 % (1) |
| Blue hue (`blue_hue`) | `blue_hue` | degrees | as Red hue's | ±20° | 0.1° | 0° |
| Blue purity (`blue_purity`) | `blue_purity` | as Red purity's | as Red purity's | −50 % to +50 % | 1 % | 0 % (1) |
| Tint hue (`achromatic_tint_hue`) | `achromatic_tint_hue` | degrees, from the red primary's direction | −180° to 180° | the range | 1° | 0° |
| Tint purity (`achromatic_tint_purity`) | `achromatic_tint_purity` | %, 100 × purity | 0 % to 99 % (0 to 0.99) | 0 % to 20 % | 0.1 % | 0 % |

A hue turns its primary about the white, and a purity moves it towards the white or past the gamut's edge. The tint moves the white towards its hue by its purity, a fraction of the way to the gamut's edge, so greys take that hue.

The promises, the tooltips, are darktable's in British English: a primary's hue "Shifts red towards yellow or magenta" and the like, its purity "Red primary purity", and the tint's its labels, as darktable's tooltips are. A script sends `SetControl { module: "primaries", control: "red_hue", value: 10, done: true }`, and `apertrust-cli edit --set primaries.red_hue=10` does the same.

**The tint's hue changes nothing while its purity is 0** (row P6), and stays enabled there, as darktable's does: darktable draws it as any other slider.

**Presets:** none: darktable has none for this module.

### Sliders against darktable

Audited against darktable's `src/iop/primaries.c`, its params, `_setup_hue_slider`, `_setup_purity_slider` and `gui_init` ([README.md](README.md), Sliders against darktable). darktable's side is in its parameters' units, with the factor and offset its slider shows them by; Apertrust's is the table above, in the units shown. `its_sliders_are_darktable_s`, in `crates/modules/src/rgb_primaries.rs`, holds the definition to this table, and the binding's factor and offset to darktable's.

| Sliders | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Red, green and blue hue | −π to π; ±`deg2radf(20)` | °, × `RAD_2_DEG` | 1 | 0.1° | No |
| Red, green and blue purity | 0.01 to 5; 0.5 to 1.5 | %, × 100 − 100 | 1 | 1 % | No |
| Tint hue | −π to π; none | °, × `RAD_2_DEG` | 1 | 1° | No |
| Tint purity | 0 to 0.99; 0 to 0.2 | %, × 100 | 1 | 0.1 % | No |

The tracks span the soft ranges: a hue's ±20°, a primary's purity −50 % to +50 %, the tint's purity 0 % to 20 %. A value typed or scripted beyond widens the track to show it, until the row is reset, as darktable's does.

The soft range also keeps a drag away from the hues that put the three primaries on one line, as darktable's does. At a purity of 0 %, red turned past +120.7° or −114.5° renders black (`docs/ports/primaries.md`, Precision), which only a typed or scripted value reaches.

## Invariants

A switched module's rows (decision 32; [README.md](README.md), Rows every table has), and P6 for the tint's hue. No control's unit is Apertrust's: degrees and the percentages are darktable's own display of its parameters, and P4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| P0 | None: the module off, untouched or with every control kept | | The plan's instances and cache keys; the frame, on one pipeline that rgb primaries joins and leaves on the way | Instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| P1 | The first step | The module on at darktable's defaults, the operation's: a control written at its default | Frame, ΔE2000 against no edit: all of it, inside the gamut and within 80 % of its saturation | darktable's behaviour: at its defaults the operation's matrix leaks red into blue and green, B + 0.0024 (R − B) and G + 0.000025 (R − G), greys untouched, as darktable's does (`docs/ports/primaries.md`, Per pixel) | Measured and reported, without a bound (decision 22) | Ramp, sweep; the chart; every corpus raw |
| P2 | Each control, moved and reset | Red hue 0°, a write at its default; red hue +10°; Green purity +25 %; Blue hue −15° with Blue purity −30 %; Tint hue 120° with Tint purity 10 %; all eight at test 0152's values as its sliders show them; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan; the frame | Each move changes the frame; reset control by control, the module stays on and renders as P1's; the module's reset turns it off and renders as no edit, rgb primaries leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| P3 | Two controls, in either order | Red hue 9.5° and Tint purity 13.4 %; Green purity +23.5 % and Blue hue 13.1° | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| P4 | Every control, the darktable match | The values of darktable's test 0152-rgb-primaries, the golden case `primaries`: each field × factor + offset, unrounded | The parameters the module binds, and the kernel's inputs through the operation's `prepare` | darktable's parameters, and so the inputs the golden harness checks against darktable's render | One f32 step on a parameter and on a setting's uniform word; 2.4e-7, two f32 steps of 1, on a matrix entry; every other word exact | The case's parameters, on the CPU |
| P5 | Each control on a layer, Phase 4 | P2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As P2 | Sweep |
| P6 | Tint hue, while Tint purity is 0 | −180°, −33.2°, 90°, 180° | The frame's pixels, against the first step's | The same: the white moves by the tint's purity alone, so the matrix is the defaults' | Exact, the pixels; the cache keys differ, as the parameter does | Sweep |
| P7 | On at darktable's defaults | Every control's default made explicit; the module on at its defaults, and off | The parameters bound (`check::switched_defaults`); the plan | Its defaults bind the operation's own exactly; on at its defaults rgb primaries is planned with no parameter set, darktable's new instance; off, it is not planned | Exact | The CPU, a colour raw's context and an image that is not a raw |
| P8 | Version 1's edits | Version 1 entries read from `.apxmp`: Red hue +10°; Tint hue 120° and Tint purity 10 % with Green purity 0 %; Tint hue −33.2° alone; and Red purity 0 % alone, and nothing | The module's state; the parameters bound | Those with a value off its default read on and bind what version 1 bound; the others read off, their values dropped, rgb primaries not planned | Exact | The CPU |

P4 is how the module matches darktable. The golden case checks the operation against darktable-cli at the case's settings, a ΔE2000 mean of 0.0000, max 0.0002 (`docs/ports/primaries.md`), and P4 holds the module's controls set to those settings to bind those parameters.

Its first step, P1, is the operation's own behaviour at its defaults, which the port matched to darktable's render of them within a mean ΔE2000 of 0.0000, max 0.0002, a local check (`docs/ports/primaries.md`, Per pixel).

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/rgb_primaries.rs`. The synthetic rows and P4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS` and the chart's with `APERTRUST_TEST_IMAGES`, both `#[ignore]`d.

| Row | Result |
| --- | --- |
| P0 | Holds, exactly. Frames byte-identical to no edit, the module off and untouched or with every control kept, on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that rgb primaries joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| P1 | Greys stay where they were; colours with far more red than blue move a little, as darktable's first step moves them (below) |
| P2 | Holds, exactly, on the sweep and on the small raw through the viewport, and on the 90D through the viewport and the export: every move changes the frame, a write at the default among them; reset control by control, the module renders as on at darktable's defaults; the module's reset renders as no edit |
| P3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| P4 | Holds: six of the eight parameters exactly, two one f32 step off (below) |
| P6 | Holds, exactly, at each of the four hues |
| P7 | Holds |
| P8 | Holds, exactly |

P1's ΔE2000 against P0:

| Image | Mean | Max | Notes |
| --- | --- | --- | --- |
| Ramp | 0.0000 | | the defaults' matrix keeps every row's sum at 1 |
| Sweep | 0.0177 | 0.571 | none over 1.0 |
| Sweep, inside the gamut | 0.0153 | 0.570 | |
| Sweep, within 80 % of the gamut's saturation | 0.0110 | 0.179 | |
| Chart | 0.0253 | 0.380 | none over 1.0 |
| The 31 corpus raws | 0.0078 (Fujifilm X100VI) to 0.0675 (Nikon D5100) | 0.659, the largest, on the Nikon Z 9 | no pixel over 1.0 |

P4 in detail: six of the eight parameters bind darktable's exactly. The tint's hue and red's purity, set to 33.200005° and −15.699997 %, bind one f32 step from darktable's, from the f32 round trip of field to control and back. That is acceptable, since darktable's own slider rounds a value to its displayed grid. The kernel's matrix is within 2.2e-8 of the golden case's in every entry, the settings' words within one step, every other word the same.

## In the panel

- **No tabs and no group labels:** darktable's eight sliders, one under another, in its order.
- **Tracks** are plain, filled from the default, where darktable paints each slider in the colours its primary takes ([Gaps](#gaps)).
- **Values** show one decimal with their unit, in the language's number format: "+9.5°", "−15.7%". A slider whose range spans zero is signed, as darktable's is, zero included: "+0.0°".
- **Keyboard:** every row is a `SliderRow`, stepping by darktable's step, Shift ten steps and Alt a tenth, never less than 0.1, the decimal shown, as darktable's `min_visible`; the step grows as a widened track's span does, as darktable's zoom step makes it. The value is rounded to the decimal shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **Painted tracks.** darktable paints each hue slider in the colours its primary turns through, the tint's from red's direction, and each purity slider from the white to its primary at the hue set (`_paint_hue_slider`, `_paint_purity_slider`), from the working and display profiles. The design system's hue track is one fixed band of hues, which would show red at 0 on green's and blue's hues, and no track follows another control's value. The rows are plain meanwhile.
2. **Hues past ±180°,** across the renderer. darktable wraps a hue outside ±180° back onto the turn: a typed value, and the tint's hue's key and scroll steps, since its slider spans the whole turn. Apertrust's slider rows clamp a value to the range's end.
3. **The module's tooltip,** darktable's "adjustment of the RGB color primaries for color grading": a definition implies no message for the module's own tooltip.

## Design choices

- **Control ids:** darktable's parameter names, `achromatic_tint_hue` and `achromatic_tint_purity` among them, so `--set` and scripts name what darktable's history names, as Colour balance RGB's do.
- **Units:** stored as shown, with darktable's slider factor and offset, the binding converting back in f32 as darktable's slider does.
- **Ranges:** darktable's hard ranges in the units shown, so every value darktable's sliders take when typed fits.
- **Steps and decimals:** darktable's: its computed steps over the soft ranges, and one decimal.
- **Tracks:** plain, filled from the default, since the design system's hue track is wrong for green's and blue's hues ([Gaps](#gaps), 1).
- **Row ids:** P, for rgb Primaries; C is Colour calibration's.
- **Row P6:** that the tint's hue changes nothing while its purity is 0, measured exactly.
- **The files' names:** by the module's English name, `rgb_primaries.rs` and this page, `rgb-primaries.md`, apart from the operation's `primaries` (the `tool-bindings` skill).
