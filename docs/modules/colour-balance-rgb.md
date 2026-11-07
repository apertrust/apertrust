# Colour balance RGB

Module `colorbalancergb`, version 1. It works on layers and has no Auto.

It is darktable's color balance rgb in the first pass (decision 2). It has darktable's page: the Master, 4 ways and Masks tabs as content tabs, and the controls in `gui_init`'s order under darktable's labels in British spelling (decision 29).

It drives one operation, `colorbalancergb` ([../ports/colorbalancergb.md](../ports/colorbalancergb.md)). An unedited photo does not run it: it is planned only while a control is off its default, as in darktable, where color balance rgb is off until the user turns it on (decision 32). The module owns every one of the operation's parameters; none is fixed.

**Status:** built, in `crates/modules/src/colour_balance_rgb.rs`. The app draws it from its definition (`docs/gui.md`, Module panels from definitions) in the Colour tab, after Output colour profile and before RGB primaries, in [README.md](README.md)'s order (decision 38).

- Its rows are tests in `crates/modules/tests/colour_balance_rgb.rs`; [Results](#results) gives what they measured.
- Contrast and its grey fulcrum are this module's, not Exposure's (decision 2): darktable's parameters, `contrast` as darktable's percentage and `grey_fulcrum` at its 18.45 % (decision 22). Brightness is this module's Power luminance, `midtones_y`.
- The first step is row B1, measured and reported without a bound (decision 22).
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

**Planned: a switched module** (decision 32). Colour balance RGB becomes switched: off until a control changes, then on with darktable's defaults for every control not set, until its reset turns it off ([README.md](README.md), rule 4).

- Its defaults are darktable's already, so its controls and binding stay as they are.
- It becomes version 2: a version 1 edit migrates on where a value was off its defaults, which is when version 1 planned the operation, its values otherwise dropped.
- Its rows become a switched module's ([README.md](README.md), Rows every table has). B0 becomes off; B1 measures what it measures today, the operation at its defaults, as the module on and untouched; B2's resets follow the switched rows; new rows after B5 hold on at darktable's defaults and version 1's edits.

The rest of this page describes the module as built.

## Controls

Each control is one `colorbalancergb` parameter, shown as darktable's slider shows it.

- **A percentage** is the parameter times 100, as darktable's sliders show their factors. The binding divides it by 100 in f32, within one f32 step of the decimal it shows: 18.45 % is 0.18450001 where darktable's default is 0.1845. The binding never writes that default, since a control at its default sets nothing.
- **Degrees and EV** are the parameter as it is.
- **Every slider** shows two decimals, as darktable's do. Its range is darktable's hard range, what a value may hold; its track spans darktable's soft range; its step is darktable's, what `dt_bauhaus_slider_get_step` gives over the soft range ([Sliders against darktable](#sliders-against-darktable)).

Control ids are darktable's parameter names in snake_case.

| Tab | Group label | Control | Parameter | Range | Soft range: the track's span | Step | Default | Track |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Master | — | Hue shift (`hue_angle`) | `hue_angle` | −180° to 180° | the range | 1° | 0 | plain |
| Master | — | Global vibrance (`vibrance`) | `vibrance` | −100 % to 100 % | ±50 % | 0.5 % | 0 | plain |
| Master | — | Contrast (`contrast`) | `contrast` | −100 % to 100 % | ±50 % | 0.5 % | 0 | plain |
| Master | Linear chroma grading | Global chroma (`chroma_global`) | `chroma_global` | −100 % to 100 % | ±50 % | 0.5 % | 0 | plain |
| Master | Linear chroma grading | Shadows, Mid-tones, Highlights (`chroma_shadows`, `_midtones`, `_highlights`) | the same | −100 % to 100 % | the range | 1 % | 0 | plain |
| Master | Perceptual saturation grading | Global saturation; Shadows, Mid-tones, Highlights (`saturation_global`, `_shadows`, `_midtones`, `_highlights`) | the same | −100 % to 100 % | the range | 1 % | 0 | plain |
| Master | Perceptual brilliance grading | Global brilliance; Shadows, Mid-tones, Highlights (`brilliance_global`, `_shadows`, `_midtones`, `_highlights`) | the same | −100 % to 100 % | the range | 1 % | 0 | plain |
| 4 ways | Global offset | Luminance, Hue, Chroma (`global_y`, `global_h`, `global_c`) | `global_Y`, `global_H`, `global_C` | −100 % to 100 %; 0° to 360°; 0 % to 100 % | ±5 %; the range; 0 % to 1 % | 0.05 %; 1°; 0.01 % | 0 | luminance; hue, unfilled; saturation |
| 4 ways | Shadows lift | Luminance, Hue, Chroma (`shadows_y`, `shadows_h`, `shadows_c`) | `shadows_Y`, `shadows_H`, `shadows_C` | as Global offset's | the range; the range; 0 % to 50 % | 1 %; 1°; 0.5 % | 0 | as Global offset's |
| 4 ways | Highlights gain | Luminance, Hue, Chroma (`highlights_y`, `highlights_h`, `highlights_c`) | `highlights_Y`, `highlights_H`, `highlights_C` | as Global offset's | ±50 %; the range; 0 % to 20 % | 0.5 %; 1°; 0.1 % | 0 | as Global offset's |
| 4 ways | Power | Luminance, Hue, Chroma (`midtones_y`, `midtones_h`, `midtones_c`) | `midtones_Y`, `midtones_H`, `midtones_C` | as Global offset's | ±25 %; the range; 0 % to 10 % | 0.1 %; 1°; 0.1 % | 0 | as Global offset's |
| Masks | — | Saturation formula (`saturation_formula`) | `saturation_formula` | JzAzBz (2021), darktable UCS (2022), in darktable's order, both ported | — | — | darktable UCS (decision 22) | a dropdown |
| Masks | Luminance ranges | Shadows fall-off (`shadows_weight`) | `shadows_weight` | 0 % to 300 % | the range | 1 % | 100 % | plain |
| Masks | Luminance ranges | Mask middle-grey fulcrum (`mask_grey_fulcrum`) | `mask_grey_fulcrum` | 0 % to 100 % | the range | 1 % | 18.45 % | plain |
| Masks | Luminance ranges | Highlights fall-off (`highlights_weight`) | `highlights_weight` | 0 % to 300 % | the range | 1 % | 100 % | plain |
| Masks | Threshold | White fulcrum (`white_fulcrum`) | `white_fulcrum` | −16 to 16 EV | ±2 EV | 0.01 EV | 0 EV | plain |
| Masks | Threshold | Contrast grey fulcrum (`grey_fulcrum`) | `grey_fulcrum` | 0 % to 100 % | 10 % to 50 % | 0.5 % | 18.45 % | plain |

The group labels are darktable's section labels, plain labels: darktable has no collapsible section in this module. A full name, what screen readers say, adds the group to a short label that repeats ("Shadows chroma", "Global offset hue"), as darktable's shortcuts do.

The promises, the tooltips, are darktable's tooltips in British English; the per-range saturation and brilliance tooltips, which darktable words alike, name their range.

`contrast` is Yrg luminance's power around `grey_fulcrum`, `1 + contrast`, at constant Yrg chromaticity; `grey_fulcrum` 18.45 % keeps RGB grey 0.1744 fixed, and RGB 0.18 is 19.04 % (`docs/ports/colorbalancergb.md`, Tool binding notes). A script sends `SetControl { module: "colorbalancergb", control: "contrast", value: 20, done: true }`, and `apertrust-cli edit --set colorbalancergb.contrast=20` does the same.

**Presets:** none yet. darktable's basic colourfulness presets (legacy, natural skin, vibrant colours, standard; the legacy one JzAzBz, the others darktable UCS) come with the header's presets menu (Phase 3), all of whose values the module takes.

### Sliders against darktable

Audited against darktable's `src/iop/colorbalancergb.c`, its params and `gui_init` ([README.md](README.md), Sliders against darktable). darktable's side is in its parameters' units:

- A percentage is shown by the "%" format, whose factor of 100 takes two of the four digits `gui_init` sets, so it shows two.
- A hue, the hue shift and White fulcrum show their parameter as it is, with the two digits `dt_bauhaus_slider_from_params` gives.

Apertrust's side is the Controls table above, in the units shown. `its_sliders_are_darktable_s`, in `crates/modules/src/colour_balance_rgb.rs`, holds the definition to this table.

| Sliders | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Hue shift | −180 to 180; none | ° | 2 | 1° | No |
| Global vibrance, Contrast, Global chroma | −1 to 1; ±0.5 | % | 2 | 0.5 % | No |
| Shadows, Mid-tones and Highlights chroma; the saturation and brilliance sliders | −1 to 1; none | % | 2 | 1 % | No |
| Global offset luminance, hue, chroma | −1 to 1, 0 to 360, 0 to 1; ±0.05, none, 0 to 0.01 | %, °, % | 2 | 0.05 %, 1°, 0.01 % | No |
| Shadows lift luminance, hue, chroma | −1 to 1, 0 to 360, 0 to 1; ±1, none, 0 to 0.5 | %, °, % | 2 | 1 %, 1°, 0.5 % | No |
| Highlights gain luminance, hue, chroma | −1 to 1, 0 to 360, 0 to 1; ±0.5, none, 0 to 0.2 | %, °, % | 2 | 0.5 %, 1°, 0.1 % | No |
| Power luminance, hue, chroma | −1 to 1, 0 to 360, 0 to 1; ±0.25, none, 0 to 0.1 | %, °, % | 2 | 0.1 %, 1°, 0.1 % | No |
| Shadows and Highlights fall-off | 0 to 3; none | % | 2 | 1 % | No |
| Mask middle-grey fulcrum | 0 to 1; none | % | 2 | 1 % | No |
| White fulcrum | −16 to 16; ±2 | EV | 2 | 0.01 EV | No |
| Contrast grey fulcrum | 0 to 1; 0.1 to 0.5 | % | 2 | 0.5 % | No |

The tracks span the soft ranges, so Global offset chroma's drag runs over 1 %, Contrast's over ±50 % and Shadows lift chroma's over 50 %, as darktable's do. A value typed or scripted beyond widens the track to show it, until the row is reset.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has). No control's unit is Apertrust's: the percentages are darktable's own display of its factors, and B4 holds the module's values to darktable's parameters.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| B0 | None: every control at its default, written at it or kept there | | `bind`; the plan's instances and cache keys; the frame, on one pipeline that colour balance rgb joins and leaves on the way | `bind` returns nothing; instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| B1 | The first step | `colorbalancergb` planned at its defaults, the operation's, as any control's first move off its default plans it | Frame, ΔE2000 against B0's: all of it, inside the gamut and within 80 % of its saturation | darktable's behaviour: the operation's gamut mapping, its Yrg chroma clip and its clips of negative channels act at any setting, as darktable's do | Measured and reported, without a bound (decision 22) | Ramp, sweep; the chart; every corpus raw |
| B2 | Each control, moved and reset | Contrast +20 %; Global brilliance −20 %; Global offset hue 120° with chroma 5 %; the formula JzAzBz; Global vibrance, Shadows lift luminance, Contrast grey fulcrum, White fulcrum and the formula together; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | `bind`; the frame | As B0, colour balance rgb leaving the plan | Exact | Ramp; the small raw through the viewport; 90D through the viewport and the export |
| B3 | Two controls, in either order | Contrast +20 % and Global saturation −30 %; JzAzBz and Global brilliance +10 % | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| B4 | Every control, the darktable match | The values of darktable's tests 0083 (params version 4, JzAzBz) and 0093-colorbalancergb-ucs (version 5, darktable UCS), the golden cases `colorbalancergb` and `colorbalancergb-ucs` | The parameters the module binds, and the kernel's inputs through the operation's `prepare` | darktable's parameters, and so the inputs the golden harness checks against darktable's render | One f32 step on a parameter; two f32 steps, 2.4e-7 relative, on a uniform word; the gamut table exact | The two cases' parameters, on the CPU |
| B5 | Each control on a layer, Phase 4 | B2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As B2 | Ramp |

B4 is how the module matches darktable. The golden cases check the operation against darktable-cli at the cases' settings, means of 0.0057 and 0.0018 ΔE2000 (`docs/ports/colorbalancergb.md`). B4 holds the module's controls, set to those settings, to bind exactly those parameters.

Its first step, B1, is the operation's own behaviour at its defaults, which the port matched to darktable's at those defaults within a mean ΔE2000 of 0.0016, max 0.0106, a local check (`docs/ports/colorbalancergb.md`, Per pixel).

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/colour_balance_rgb.rs`. The synthetic rows and B4 run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS` and the chart's with `APERTRUST_TEST_IMAGES`, both `#[ignore]`d.

| Row | Result |
| --- | --- |
| B0 | Holds, exactly. `bind` sets nothing with every control written at its default, or kept there. Frames byte-identical to no edit on the ramp and the sweep, and on the small raw through the export and the viewport, each on one pipeline that colour balance rgb joins and leaves on the way. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| B1 | darktable's behaviour: neutral and moderately saturated colours stay, colours near the gamut's edge move; corpus means of 0.0000 to 0.0274 ΔE2000 (below) |
| B2 | Holds, exactly, on the ramp and on the small raw through the viewport, and on the 90D through the viewport and the export: every move changes the frame, and resets render as no edit |
| B3 | Holds, exactly, on the sweep and on the 90D through the viewport and the export |
| B4 | Holds. Test 0083's kernel inputs are the golden case's byte for byte; test 0093's are within 7.5e-8 relative, one f32 step from a percentage's division, its gamut table exact |

B1 in detail, with darktable UCS, the default. Colour calibration runs on every colour raw (decision 27) from darktable's own defaults, As shot snapped to daylight or Planckian where darktable snaps, and compresses the gamut before colour balance rgb does.

- **The ramp's greys and the sweep's colours within 80 % of the gamut's saturation:** 0.000.
- **Inside the gamut:** a mean of 0.0606, 2.18 % over 1.0, max 3.92, on pure hues.
- **The whole sweep,** colours with a negative channel included: a mean of 0.148, 5.86 % over 1.0, max 3.97. The synthetic images have no camera, so no As shot to snap.
- **The chart:** a mean of 0.0051, 0.013 % over 1.0, max 2.40.
- **The 31 corpus raws:** means of 0.0000 to 0.0274, at most 0.41 % of pixels over 1.0. The Nikon D5100's mean 0.027 and 0.41 % over 1.0, the Sony ILCE-1's 0.0068 and 0.23 %. Single pixels over 3.0 on the Panasonic DC-S5M2 at 96 MP (3.33) and 24 MP (3.24), the Phase One IQ3 100MP (3.18) and the Nikon Z 9 (3.11).

So darktable UCS leaves neutral and moderately saturated colours where they were and moves colours near the gamut's edge, as darktable's first step does.

**With JzAzBz,** darktable's other formula, measured without colour calibration before it (Exposure's row E1):

- The ramp's greys a mean of 0.0013, max 0.014.
- The sweep within 80 % of the gamut a mean of 0.0012, max 0.015, none over 1.0, the largest on near-neutral colours (saturations 0.05 to 0.2), so from the operation's own f32 round trip rather than its gamut mapping. Inside the gamut a mean of 0.016, 0.34 % over 1.0, max 4.1, on pure hues. The whole sweep a mean of 0.23, 5.4 % over 1.0, max 24.
- The chart: a mean of 0.011, 0.17 % over 1.0, max 4.7.
- The 31 corpus raws: means of 0.0014 to 0.027, at most 0.27 % over 1.0. The Nikon D5100's mean 0.027, and single pixels over 3.0 on the Phase One IQ3 100MP (17.0), the Canon EOS R5 (10.8), the Panasonic DC-S5M2 at 96 MP (7.0) and 24 MP (3.9), the Canon EOS 5D Mark III's DNG (4.0) and the Sony ILCE-1 (3.5).

## In the panel

- **Tabs:** Master, 4 ways and Masks, the first shown at first; the tab shown is the app's for the run. Nothing shows under every tab.
- **Hues** are drawn on the hue track and unfilled, as darktable draws them. The luminance and saturation tracks are drawn plain until the renderer draws those bands (`docs/gui.md`, What the renderer still lacks).
- **Saturation formula** is a dropdown with both of darktable's options, both ported.
- **Keyboard:** every row but the formula is a `SliderRow`, stepping by darktable's step, Shift ten steps and Alt a tenth. A step is never less than 0.01, the last decimal shown, as darktable's `min_visible`, so Alt moves Global offset chroma, whose step is 0.01 %. The step grows as a widened track's span does, as darktable's zoom step makes it. The value is rounded to the two decimals shown before it is sent.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has that the panel does not draw yet. A row the definition cannot hold is left out of it until it can; one it can hold, as the pickers, is in it, drawn once its tool runs:

1. **The luminance ranges' graph,** darktable's read-only graph of the three masks, first under Luminance ranges on the Masks tab. The design system's closed registry has no such widget, and the renderer draws no custom row but the curve.
2. **The pickers** on the four hue sliders (darktable's "pick opposite colour from image", Ctrl+click the colour itself) and on White fulcrum and Contrast grey fulcrum. They are in the definition (`Quad::Picker`), so it holds darktable's page whole, but the renderer draws no quad until its tool runs (`docs/gui.md`, Module panels from definitions: a quad with nothing behind it).
3. **The mask display** quads on Shadows fall-off, Mask middle-grey fulcrum and Highlights fall-off, which show a range's mask on the photo as a checkerboard. The definition's `Quad` has no showmask yet, and the mask preview is a view mode, later ([README.md](README.md), rule 10).
4. **Mask preview settings,** darktable's last section label with its two checkerboard colours and size: darktable's settings, the mask preview's app settings when it comes (rule 12). The size could be drawn disabled as a `Row::ToolSetting` (`docs/gui.md`, Module panels from definitions), but not the two colours, since a tool setting has no colour kind.
5. **Tooltips with no message:** darktable's tabs' ("global grading", "selective color grading", "isolate luminances"), and a module tooltip mentioning the first step's shift, which [README.md](README.md) asks for: a definition implies no message for either, so this document says it (B1).
6. **Tracks:** darktable paints a wheel's chroma track in that wheel's hue, which a track following another control's value would need; the luminance and saturation bands are drawn plain meanwhile.

darktable changes no row's range by mode in this module, so the renderer's missing per-mode ranges (`docs/gui.md`, What the renderer still lacks) do not touch it.

## Design choices

- **Control ids:** darktable's parameter names in snake_case, `shadows_y` for `shadows_Y`, the rest as darktable's, so `--set` and scripts name what darktable's history names.
- **Ranges:** darktable's hard ranges, in the unit shown, so every value darktable's sliders take when typed, and every preset, fits.
- **Percentages:** stored as shown, the binding dividing by 100 in f32.
- **Steps:** darktable's, from its soft ranges.
- **Row ids:** B, for colour Balance RGB, leaving C for Colour calibration.
- **The files' names:** by the module's English name, as the `tool-bindings` skill has it for every module: the source and tests `colour_balance_rgb.rs`, snake_case, this page kebab-case, apart from the operation's `colorbalancergb`.
- **Exposure's Contrast in an edit:** left out with a warning when read. Exposure stays version 1, and nothing migrates to this module, since no released edit holds it.
