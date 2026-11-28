# Colour equalizer

Tool `colorequal`, version 1. It works on layers, has no Auto and is switched (decision 32).

Its header's tooltip says what it is for (`tool-colorequal-description`, [gui.md](../gui.md#the-headers-tooltip)): "Changes the hue, saturation and brightness of each range of hues on its own. Reach for it to turn greens towards yellow, deepen blues or brighten skin."

It is darktable's color equalizer in the first pass (decision 2): darktable's page in its default layout, each curve's nodes on its graph, its controls in `gui_init`'s order under darktable's labels in British spelling (decision 29).

It drives one operation, `colorequal` ([../ports/colorequal.md](../ports/colorequal.md)), which an unedited photo does not run. The operation is planned while the tool is on, as in darktable, where color equalizer is off until the user turns it on (decision 32). The tool owns all 32 of the operation's parameters; none is fixed.

**Status:** built, in `crates/tools/src/colour_equalizer.rs`. The app draws it from its definition (`docs/gui.md`, Tool panels from definitions). Its tab is the Default workspace's Adjust, after Colour balance RGB and before RGB primaries (decision 38).

- Its rows are tests in `crates/tools/tests/colour_equalizer.rs`; [Results](#results) gives what they measured.
- To come: Node placement's picker, which marks the picked hue on the graph; the show-mask quads; darktable's presets, with the presets API every tool shares ([To come](#to-come)).

**Switched** (decision 32; [README.md](README.md), rule 4). The Colour equalizer is off on a new photo, as darktable's color equalizer is, and renders nothing. No value of its is an identity: at darktable's defaults the guided filter's prefilter smooths the chromaticity the output is rebuilt from, and without the filter the curves still ripple between their nodes and the gamut mapping compresses saturations past 0.8 of the boundary (`docs/ports/colorequal.md`, Tool binding notes). Any write of a control turns it on, whatever the value, with darktable's defaults for the others, until its reset turns it off; resetting one control leaves it on. Its defaults are darktable's, the operation's own: on at its defaults, the operation runs at its own defaults, darktable's new instance.

## The page

darktable has two layouts, which a middle-click on its graph switches (`plugins/darkroom/colorequal/show_sliders`): the graph alone, its default, or the graph with each tab's sliders and the options as a fourth tab. The page is darktable's default, the graph alone, laid out as the tool model holds darktable's rows outside its notebook (`before` and `after`, [gui.md](../gui.md)):

- **Tabs:** Hue, Saturation and Brightness, darktable's notebook, each darktable's graph of its curve's eight nodes from red to magenta ([The graph](#the-graph)), their only row.
- **Under every tab:** Node placement, the nodes' hue moved together, after the tab's graph.
- **Options,** collapsible, under every tab, after Node placement: White level, Hue curve, Use guided filter, then Hue analysis radius, Saturation threshold, Contrast and Effect radius, shown only while the guided filter is in use, as darktable's `gui_changed` shows them (`Row::When`). darktable shows Hue curve among its options on the Hue tab alone.

## The graph

Each tab is darktable's graph of its curve, `Widget::HueCurve` over the tab's eight nodes ([gui.md](../gui.md#the-hue-curve)), their only row (decision 31): a node is set by the pointer, the wheel or the keys, and a screen reader reaches each as a slider with its value, range and step.

- **Across,** the hue: each node at its hue, `k · 45° + Node placement` from red, red drawn again at the far end; under the graph a strip of each hue at the graph's saturation.
- **Up,** the node's range, its default in the middle: −180° to +180° on Hue, −100 % to +100 % on Saturation and Brightness.
- **The curve** is the operation's (`ProcessingTool::curve`), darktable's periodic RBF through the nodes, with Hue curve's smoothing on Hue, clipped at no gain on the others, from `apertrust-color` as the operation's setup has it.
- **Behind it,** darktable's backgrounds (`ProcessingTool::curve_paint`): on Hue each hue turned by the height, as a node there turns it; on Saturation from grey to full; on Brightness from black to 1.25 times the sliders' brightness.
- **A press on a node** and a drag move it to the pointer's height; a double-click resets it, or every node of the tab off the nodes; the wheel steps the nearest node as a slider row's wheel does.

## Controls

Each control is one `colorequal` parameter, shown as darktable's slider shows it: a saturation or brightness node as 100 times its gain less 100, so darktable's 1 is 0 %; Saturation threshold as 100 times its parameter; every other control as it is. The binding goes back as darktable's slider does with a typed value, `(value − offset) / factor` in f32. Control ids are darktable's parameter names.

| Control | Parameter | Shown as | Range | Soft range: the track's span | Step | Default |
| --- | --- | --- | --- | --- | --- | --- |
| Hue: Red to Magenta (`hue_red` … `hue_magenta`) | `hue_red` … | degrees | −180° to 180° | the range | 1° | 0° |
| Saturation: Red to Magenta (`sat_red` … `sat_magenta`) | `sat_red` … | %, 100 × gain − 100 | −100 % to +100 % (0 to 2) | the range | 1 % | 0 % (1) |
| Brightness: Red to Magenta (`bright_red` … `bright_magenta`) | `bright_red` … | as Saturation's | as Saturation's | the range | 1 % | 0 % (1) |
| Node placement (`hue_shift`) | `hue_shift` | degrees, no decimals | −23° to 23° | the range | 0.1° | 0° |
| Hue curve (`smoothing_hue`) | `smoothing_hue` | as it is | 0.05 to 2 | the range | 0.01 | 1 |
| White level (`white_level`) | `white_level` | EV | −2 to 16 EV | −2 to +2 EV | 0.01 EV | 1 EV |
| Use guided filter (`use_filter`) | `use_filter` | a toggle | | | | on |
| Hue analysis radius (`chroma_size`) | `chroma_size` | px | 1 to 10 px | the range | 0.1 px | 1.5 px |
| Saturation threshold (`threshold`) | `threshold` | %, 100 × parameter | 0 % to 30 % (0 to 0.3) | the range | 0.1 % | 10 % |
| Contrast (`contrast`) | `contrast` | as it is | −1 to 1 | the range | 0.01 | 0 |
| Effect radius (`param_size`) | `param_size` | px | 1 to 128 px | the range | 1 px | 1 px |

The nodes sit at hues `k · 45° + Node placement + 20°` in darktable UCS, red first. A hue node turns that range of hues, a saturation node scales its saturation and a brightness node its brightness, each spread between the nodes along a periodic curve whose sharpness Hue curve sets. White level, in EV, caps the brightness corrections. The guided filter limits the corrections to areas of one colour, weighted by saturation so greys are left alone.

The promises, the tooltips, are darktable's in British English where it has them: Node placement "Shift nodes to lower or higher hue", White level "The white level set manually or via the picker restricts brightness corrections…", and the guided filter's; darktable gives the nodes none, so each says what it changes ("Turns the hue of red tones."). A full name says the node's tab, as darktable's shortcuts do: "Hue: red". A script sends `SetControl { tool: "colorequal", control: "sat_blue", value: 25, done: true }`, and `apertrust-cli edit --set colorequal.sat_blue=25` does the same.

**White level's picker** is darktable's, an area: the area's brightest luminance in EV, darktable's `log2f` of its maximum's Y (`color_picker_apply`). It is in the definition, as darktable's page has it; the renderer draws its quad once what it runs is built (decision 79, [pickers.md](../pickers.md)).

**Presets:** darktable's, all version 4 with the guided filter, come with the presets API every tool shares.

### Sliders against darktable

Audited against darktable's `src/iop/colorequal.c`, its params, `gui_init` and `_init_sliders`, and `dt_bauhaus_slider_from_params` in `src/develop/imageop_gui.c` ([README.md](README.md), Sliders against darktable). `its_sliders_are_darktable_s`, in `crates/tools/src/colour_equalizer.rs`, holds the definition to this table, and the binding's factor and offset to darktable's. The nodes have no slider of their own: their graph draws each over darktable's range, steps it by darktable's step and rounds it to its digits.

| Sliders | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Hue nodes | −180 to 180; none | ° | 2 | 1° | No |
| Saturation and brightness nodes | 0 to 2; none | %, × 100 − 100 (`_init_sliders`' format and offset) | 2 | 1 % | No |
| Node placement | −23 to 23; none | °, `set_digits(0)` | 0 | 0.1° | No |
| Hue curve | 0.05 to 2; none | as it is | 2 | 0.01 | No |
| White level | −2 to 16; −2 to 2 | EV | 2 | 0.01 EV | No |
| Hue analysis radius | 1 to 10; none | px, `set_digits(1)` | 1 | 0.1 px | No |
| Saturation threshold | 0 to 0.3; none | %, × 100: `set_digits(3)`, then the "%" format takes two | 1 | 0.1 % | No |
| Contrast | −1 to 1; none | as it is, `set_digits(3)` | 3 | 0.01 | No |
| Effect radius | 1 to 128; none | px, `set_digits(1)` | 1 | 1 px | No |

## Invariants

A switched tool's rows (decision 32; [README.md](README.md), Rows every table has). No control's unit is Apertrust's: the degrees, EV, pixels and percentages are darktable's own display of its parameters, and H4 holds the tool's values to darktable's parameters. The tool is new at version 1, so it has no older edit's row.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| H0 | None: the tool off, untouched or with every control kept | | The plan's instances and cache keys; the frame, on one pipeline that colorequal joins and leaves on the way | Instances, keys and frames as with no edit | Exact | Ramp, sweep, the small raw through the export and the viewport; every corpus raw |
| H1 | The first step | The tool on at darktable's defaults, the operation's: a control written at its default | Frame, ΔE2000 against no edit: all of it, inside the gamut and within 80 % of its saturation | darktable's behaviour: greys untouched; colours moved by the prefilter and the gamut mapping, as darktable's are | Measured and reported, without a bound (decision 22) | Ramp, sweep; the chart; every corpus raw |
| H2 | Each control, moved and reset | Hue: Red 0°, a write at its default; Hue: Green 30°; Saturation: Blue +40 %; Brightness: Orange +50 % with Node placement 10°; Hue: Yellow −20° with Hue curve 0.5 and White level −1 EV; Use guided filter off with Saturation: Red −50 %; Saturation: Cyan +60 % with the guided filter's four settings; each reset control by control (`ResetControl`) and by the header's reset (`ResetTool`) | The plan; the frame | Each move changes the frame; reset control by control, the tool stays on and renders as H1's; the tool's reset turns it off and renders as no edit, colorequal leaving the plan | Exact | Sweep; the small raw through the viewport; 90D through the viewport and the export |
| H3 | Two controls, in either order | Hue: Green 30° and Saturation: Blue +40 %; Brightness: Orange +50 % and Use guided filter off | Frame | The same whichever was set first | Exact | Sweep; 90D through the viewport and the export |
| H4 | Every control, the darktable match | The values of darktable's tests 0158-coloreq-no-guided-filter and 0159-coloreq-guided-filter, the golden cases `colorequal-no-filter` and `colorequal`, migrated to version 4 as darktable's `legacy_params` does, each shown as its slider shows it, unrounded | The parameters the tool binds; the passes and uniforms the operation's `prepare` gives from them | darktable's parameters, and so the inputs the golden harness checks against darktable's render | One f32 step on a parameter; four on a uniform | The cases' parameters, on the CPU |
| H5 | Each control on a layer, Phase 4 | H2's moves on a full-mask layer at 100 % | Frame, inside the mask | As the same values on the background | As H2 | Sweep |
| H6 | On at darktable's defaults | Every control at its default, made explicit | The parameters the tool binds | The operation's own defaults, and colorequal planned with no parameter set; off, not planned | Exact | On the CPU |

## Results

Measured on the Radeon 780M (RADV):

- **H1, the first step:** on the ramp, none, the greys untouched; on the sweep, a mean ΔE2000 of 1.01 and at most 23.4, 17.1 % of pixels over 1.0; inside the gamut 0.74, 11.9 % over 1.0; within 80 % of its saturation 0.79, 11.0 % over 1.0.
- **H4:** on both cases one parameter of the 32 is one f32 step from darktable's, the round trip's rounding of a percentage, and every uniform `prepare` gives is the same.

## Design choices

1. **darktable's default layout, the graph alone,** with no slider for each node: the graph sets every node by pointer, wheel and keys, and reads each out to a screen reader, so slider rows would only repeat it.
2. **Options under every tab,** collapsible, where darktable's sliders layout makes them a fourth tab: the tool model holds rows outside the notebook (`after`), as darktable's graph layout draws them. Hue curve, which darktable shows among them only while the Hue tab is chosen, is there on every tab, after White level, so it is in one place whichever tab is chosen.
3. **Node placement under every tab** (`after`), after the graph whose nodes it moves and before Options, as darktable draws it under its graph: each tab draws its own graph, so the graph sits under the tab strip.
4. **The graph's height is each node's range,** so a hue node follows the pointer. darktable's drag sets a hue node to `(0.5 − y) · 349°` (`_area_set_value`), reaching ±174.5° at the graph's edges while it draws ±180° there.
5. **The backgrounds run with the strip.** darktable's `_init_graph_backgrounds` runs each column's hue backwards, 270° to −90°, and writes red, green and blue into cairo's `RGB24`, which holds them blue first, two faults that nearly cancel. Here each column is the strip's hue.
6. **sRGB.** darktable paints the colours in the display's profile; they are painted in sRGB, darktable's own fallback (`dt_XYZ_to_sRGB`), until the display's colour reaches the panel.
7. **No middle-click, Ctrl+click or Alt+wheel on the graph:** the page has the one layout, and the tab strip is the tool's.

## To come

- **Node placement's picker,** which only marks the picked hue on the graph (`_draw_color_picker`).
- **The show-mask quads** of Saturation threshold and Effect radius, with Show mask (Phase 3).
- **darktable's presets,** with the presets API.
- **The viewport's target** on four corpus renders, which `docs/engine.md` puts before the tool ships ([Pipes](../engine.md)).
