# Tone curve

Module `tone_curve`, version 1. It works on layers and has no Auto.

It drives one `rgbcurve` instance. An unedited photo does not run it, so it is planned only while the curve is off the identity (decision 28). The identity curve is untouched.

**Decision 2** (rewrite first, improve later): the first pass shows darktable's RGB curve and RGB levels as modules of their own ([README.md](README.md)). The built Luma curve below becomes RGB curve's linked curve, as separate work, planned:

- `luma_points` becomes its points, `preserve_colour` its preserve colours, and `LumaCurve` the curve it draws.
- `tone_curve` then joins `RETIRED`, so an edit holding it loses those values with a warning (decision 43).

**Status:** the curve runs at darktable's place, `rgbcurve` at 42, before the tone mapper (decision 22).

- A tolerance or a choice marked proposed is open.
- The rows measure at the module's input and output.
- Measures and test images are in [README.md](README.md).

**Built: the Luma curve and Preserve colours,** in `crates/modules/src/tone_curve.rs`. The module renderer draws it with the design system's curve in the app's Base tab, at RGB curve's place: first of the built modules there, before the Tone equalizer (decision 38). What is built:

- **Controls:** `luma_points` holds the points between the curve's ends, up to 18, x at least 0.0025 apart and as far from the ends (as `rgbcurve` and the curve widget keep them), none by default, the identity. The ends stay at (0, 0) and (1, 1) and are not stored: the binding adds them as the curve's first and last nodes (decision 31). The panel draws them as points that do not move.
- **Preserve colours:** `preserve_colour` is one choice over six of `rgbcurve`'s norms (`docs/ports/rgbcurve.md`, Tool binding notes): luminance the default, then max RGB, average RGB, sum RGB, norm RGB and basic power, by the port's names (`luminance`, `max_rgb`, `average_rgb`, `sum_rgb`, `norm_rgb`, `basic_power`).
- **Tabs:** Luma, RGB, Red, Green, Blue and Advanced, Preserve colours on the Advanced tab. RGB, Red, Green and Blue have no rows, so the panel draws them disabled; they give way to RGB curve's mode and channel tabs when the module becomes RGB curve.
- **Binding:** one `rgbcurve` instance, linked, monotone Hermite, middle grey compensated, through (0, 0), the points and (1, 1), preserving colours by the choice. Nothing while the curve is straight, every point on the diagonal, whatever Preserve colours holds. The module owns every `rgbcurve` parameter.
- **What the panel draws** is the curve `rgbcurve` renders for greys read by their value, on lightness axes (`Module::curve`, `tone_curve::LumaCurve`): the nodes made linear, darktable's monotone Hermite tangents and darktable's power law past the last, read back as lightness. A straight curve draws the diagonal exactly, so a point added on it keeps it straight. With Preserve colours on sum or norm, greys do not render as drawn (T4 and T17, below); what to draw then is open.
- **Rows:** T0, T1, T4 to T8, T11, T16 and T17 are tests in `crates/modules/tests/tone_curve.rs`, T4, T5 and T17 for each Preserve colours option ([Results](#results)).

## Controls

| Control | Kind and range | Default | Changes | Keeps |
| --- | --- | --- | --- | --- |
| `luma_points` | Curve points, `x,y` pairs in the curve's axis units, 0 to 1. At most 18 points, x strictly increasing and at least 0.0025 apart and as far from the ends | None: the identity | The tones the curve acts on, as drawn | Colour, as Preserve colours measures it |
| `preserve_colour` | Choice: luminance, max RGB, average RGB, sum RGB, norm RGB, basic power | Luminance | Which measure of each pixel the curve acts on | The curve drawn |

**The axis** is perceptual: lightness, with 0.5 at middle grey (`rgbcurve`'s `compensate_middle_grey`, always on). The curve runs at darktable's place, before the tone mapper (decision 22), so the axis's 1 is scene light at 1.0, about 2.4 EV above middle grey, and the highlights brighter than that follow the curve's extrapolation past the last node.

**The binding** (proposed):

- `rgbcurve` linked, preserving colours as `preserve_colour` says, luminance by default, monotone Hermite. Its nodes are (0, 0), the points, then (1, 1).
- `curve_type` stays monotone Hermite, the one interpolation that never overshoots between monotone points. Cubic spline and Catmull-Rom remain for presets ported from darktable.

**In the panel and from scripts:** clicking adds a point, and dragging one off the frame removes it. Everything else here is proposed, so that no interaction needs the mouse:

- With the curve focused, Page Up and Page Down select the next or previous point.
- The arrows move the selected point by 0.001, with Shift ten times that and Alt a tenth.
- Insert adds a point on the curve, midway to the next one.
- Delete removes the selected point.
- Enter opens the point's x and y value fields.
- Double-click on the curve's background resets the curve, one undo step.
- The accessible value reads the curve, the number of points and the selected point.

A script sends `SetControl` with the whole list, such as `{ module: "tone_curve", control: "luma_points", value: [[0.25, 0.2], [0.75, 0.8]] }`. The edit stores points as `x,y` pairs.

### Sliders against darktable

Audited against darktable's `src/iop/rgbcurve.c` ([README.md](README.md), Sliders against darktable). Its `gui_init` draws the curve, three lists (the mode, the interpolation and preserve colours) and a toggle, and no slider. The built Luma curve has none either, so there is nothing to compare.

`it_has_no_slider_as_darktable_s_rgb_curve_has_none`, in `crates/modules/src/tone_curve.rs`, holds the definition to that: a slider added to it fails the test until its row of darktable's is written there and here.

## Invariants

Rows that measure the module's output against its input plan the module's operations alone on the test image, with no AgX before them, so the curve sees the ramp as it is. Rows that measure the frame run the whole pipeline, AgX included, as an edit does.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| T0 | None: the curve at the identity | | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | Every image |
| T1 | Luma's first step | `rgbcurve` at the identity, linked, luminance, explicit | Module output against its input; frame, ΔE2000 against T0's | No pixel changes beyond the curve's precision | Module output within 1e-5, relative above 1; frame max ≤ 0.01 (proposed) | Every image |
| T4 | Luma points | darktable's presets "contrast medium (linear)", "contrast high (linear)" and "contrast compression", and one point at (0.25, 0.35) | Module output: each pixel's luminance on the axis | Renders as drawn: through each point, and between points as the curve's CPU twin | 2e-4 at the points, 1e-4 between them, in axis units (proposed) | Ramp; 90D |
| T5 | Luma points | As T4 | Module output: each pixel's RGB over its input's | One factor per pixel: hue and saturation are kept | Channel spread ≤ 1e-6 relative, on pixels with luminance over 1e-6 (proposed) | Sweep; chart |
| T6 | Luma points | Any curve whose points rise left to right | Module output, on the ramp | Tones keep their order; nothing overshoots between points | Exact order | Ramp |
| T7 | Adding a point | A point on the identity; a point on each of T4's curves at x 0.1, 0.5 and 0.9 | Module output, before and after | On the identity, no change. On a curve, the curve moves by at most 0.01 of the axis anywhere, as the interpolation reshapes the neighbouring segments | 1e-5; 0.01 (proposed; measure before confirming) | Ramp |
| T8 | Removing a point | Drag one off, then undo; remove the only point | Frame | Undo restores it; removing the only point returns to T0 | Exact | Ramp |
| T11 | Points, as commands | x out of order or closer than 0.0025, 19 points | The command's result; `bind` | Refused, and the widget never sends them; `bind` never returns nodes `rgbcurve`'s `prepare` refuses | Exact | CPU |
| T16 | Extremes | Curves to 0 and to 1, points 0.0025 apart, a step from 0 to 1 | Frame | Finite everywhere | No NaN or infinity | Sweep; extended ramp |
| T17 | Preserve colours | Each of the six options with T4's curves: luminance, max RGB (the largest channel), average RGB (the channels' mean), sum RGB (their sum), norm RGB (√(r² + g² + b²)) and basic power ((r³ + g³ + b³) / (r² + g² + b²)) | Module output: the option's measure of each pixel, before and after, read in f64 | The curve acts on that measure: the output's measure is the curve, as `rgbcurve` evaluates it on linear values, of the input's, and T4 and T5 hold with the option. Sum and norm measure a grey v as 3v and √3·v, so with them a grey renders C(3v)/3 and C(√3·v)/√3, not the curve drawn, and T4 cannot hold | Within 1e-5, relative, on pixels whose measure is over 1e-6 (proposed); T4's and T5's own | Ramp; sweep |
| T20 | The curve on a layer, Phase 4 | T4's curves on a full-mask layer at 100 % | Module output inside the mask | Changes the image as the same curve does on the background | As T4 | Ramp |

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/tone_curve.rs`, the Luma curve's rows only; they run on CI's software GPUs too. The corpus raws and the chart the rows name are not run yet.

Where a row fails, its proposed bound stays in an ignored test while it is open, and CI checks the parts of it that hold; the bound itself is unchanged (decision 30).

| Row | Result |
| --- | --- |
| T0 | Holds, exactly. `bind` sets nothing for the default written out, for points on the diagonal, each with every Preserve colours option, and for the controls kept at their defaults. Frames, instances and cache keys as with no edit on the ramp and the sweep, and on a small synthetic Bayer raw through the export and the viewport pipes, each on one pipeline that `rgbcurve` joins and leaves on the way |
| T1 | Holds. The module's output within 1.9e-6 of its input on the ramp, 2.6e-6 on the extended ramp and 1.1e-6 on the sweep, relative above 1. The frame against no edit, with `rgbcurve` before AgX at darktable's 42 (decision 22): a max ΔE2000 of 7.2e-5 on the small raw, 3.9e-5 on the ramp, 9.4e-5 on the sweep |
| T4 | Holds on the ramp with luminance, max RGB, average RGB and basic power. **With sum RGB and norm RGB it fails**, as T17 says it must (below) |
| T5 | Holds on the sweep with every option, for T4's curves: a channel spread of at most 1.2e-7 |
| T6 | Holds on the ramp, for T4's curves and a steep S: every grey's output in order, and none past the heights of the points either side |
| T7 | On the identity, holds exactly: the point is on the diagonal, so nothing is planned. **On a curve it fails its proposed bound of 0.01** (below) |
| T8 | Holds, exactly: removing the only point renders as no edit; setting it again, as undo will, renders as before; resetting the control and the module render as no edit. Undo itself waits for the history |
| T11 | Holds: 19 points, x out of order or 0.002 apart, a point on an end or closer to one than 0.0025, a coordinate outside 0 to 1 and NaN are refused; 18 points, and the curves above with Preserve colours options, bind to nodes `rgbcurve`'s setup takes. The ends do not move: the renderer sends nothing when one is dragged or moved by the keys. The widget's own tests hold that it never sends what is refused |
| T16 | Holds: curves to 0 and to 1 between their ends, a falling curve, points the gap apart and a step from 0 to 1, with every Preserve colours option, render finite at the module's output and in the frame, on the sweep and the extended ramp |
| T17 | Holds with every option: each option's measure of the output is the curve of its measure of the input within 5.0e-7 on the ramp and 5.7e-7 on the sweep, but luminance, 3.5e-6 on the sweep's out-of-gamut blues (hue 250°, saturation 1.1), where luminance's terms nearly cancel in the kernel's f32. It is T4 that sum and norm fail |

**T4 in detail.** The curves are darktable's "contrast medium (linear)", "contrast high (linear)" and "contrast compression", and the one point at (0.25, 0.35).

- With luminance, max RGB, average RGB and basic power: at most 8.9e-8 of the axis off at the points, the ends included, and 1.8e-7 between them, against the curve the panel draws.
- With sum RGB and norm RGB: the worst of each curve is 0.073 to 0.19 of the axis with sum and 0.044 to 0.10 with norm, both largest on contrast high.

**T7 in detail.** Adding a point re-derives its neighbours' monotone Hermite tangents, so the segments beside it reshape.

- It fails by 0.062 of the axis on contrast compression at 0.5, 0.050 and 0.046 on the one point at 0.5 and 0.9, and 0.0100023 on contrast high at 0.9.
- At 0.1 it holds on every curve (at most 0.0065), at 0.9 on the other three (at most 0.0064). Medium and high have a point at 0.5 already.

The ignored tests that keep the failing bounds: `t7_adding_a_point_on_a_curve_moves_it_within_its_proposed_bound` and `t4_luma_points_render_as_drawn_with_sum_and_norm`. CI checks T7 on the identity and at 0.1 on every curve, and T4 with the four other options.

## Phases

- **Phase 2:** the Luma curve with Preserve colours, built, with rows T0, T1, T4 to T8, T11, T16 and T17.
- **Then:** the module becomes RGB curve's linked curve (decision 2).
- **Phase 4:** layers, with T20.

The curve widget is built: `Curve` in `apertrust-ui` (`docs/gui.md`, The curve). It differs from the proposals under In the panel and from scripts in five places, each open, with its options in `docs/gui.md`:

- **The key step:** a hundredth of the plot, settable.
- **Resetting:** Shift+Delete, not a double-click on the background, which would first add a point.
- **Removing a point:** a double-click, not dragging it off the frame.
- **Enter's value fields:** none in the first pass, as darktable's curves have none (decision 42's design: a readout shows the values).
- **The accessible value:** the plot is a list of the points, each named with its input and output, rather than one value naming the curve, the count and the selected point.
