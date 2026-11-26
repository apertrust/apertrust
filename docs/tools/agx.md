# AgX

Tool `agx`, version 1. It works on the background only until layers come; with them a layer may choose it as its own tone mapper, replacing the background's within the layer's mask (decision 33). It has no Auto.

Its header's tooltip says what it is for (`tool-agx-description`, [gui.md](../gui.md#the-headers-tooltip)): "Fits the light the camera caught into what a screen or a print can show, with natural highlights and colours. The default tone mapper, inspired by Blender’s AgX."

It is darktable's AgX in the first pass (decision 2), the tone mapper of the Default workspace's Adjust tab. It has darktable's page: the Settings and Primaries tabs, the controls in `gui_init`'s order under darktable's labels in British spelling, and darktable's three collapsible sections as collapsible groups. Each row shows where darktable's `gui_changed` shows it (decision 29).

It drives one operation, `agx` ([../ports/agx.md](../ports/agx.md)), which every photo runs, and owns all 35 of its parameters, each with its control. Its defaults are the operation's, darktable's scene-referred defaults ([README.md](README.md), rule 4), so an untouched tool sets nothing.

**Status:** built, in `crates/tools/src/agx.rs`. The app draws it from its definition (`docs/gui.md`, Tool panels from definitions). Its tab is the Default workspace's Adjust, after Exposure and before the Tone equalizer, in [README.md](README.md)'s order (decision 38).

- Its rows are tests in `crates/tools/tests/agx.rs` and the tool's source, and the panel's in `crates/app/src/tools.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- One tone mapper per edit, with sigmoid and filmic rgb as tools, is planned separately.
- Measures and test images are in [README.md](README.md).

## Controls

Each control is one `agx` parameter, its id the parameter's name, shown as darktable's slider shows it: a percentage the parameter times 100, a rotation in degrees, the parameter times 180/π, as darktable's `RAD_2_DEG` factor in f32, and the pivot in EV against the range ([below](#the-pivot)). The binding divides back.

**Settings:**

| Group | Control | Parameter | Shown as | Range | Step | Decimals | Default | Shows | Ported |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| *Input exposure range*, a label | White relative exposure | `range_white_relative_ev` | EV | 0.1 to 20 EV | 0.1 EV | 2 | 6.5 EV | Always | Yes, with its picker ([below](#the-pickers)) |
| | Black relative exposure | `range_black_relative_ev` | EV | −20 to −0.1 EV | 0.1 EV | 2 | −10 EV | Always | Yes, with its picker ([below](#the-pickers)) |
| | Dynamic range scaling | `dynamic_range_scaling`, no parameter | %, × 100 | −50 % to +200 % | 0.5 % | 2 | +10 % | Always | Yes: the pickers' and Read exposure's margin, binding nothing ([below](#dynamic-range-scaling)) |
| | Auto tune levels, with Read exposure | — (a line: the `range` picker, then Read exposure's camera, an action reading Exposure's published `effective_exposure`, decision 107) | | | | | | Always | Yes ([below](#the-pickers), [Read exposure](#the-actions)) |
| *Basic curve parameters*, a label | Pivot relative exposure | `curve_pivot_x` | EV, against the range | Black to white relative exposure, −10 to +6.5 EV at the defaults, −20 to +20 EV the widest | darktable's for the range's width, 0.1 EV at the defaults | 2 | 0 EV | Always | Yes, with its picker ([below](#the-pickers)) |
| | Pivot target output | `curve_pivot_y_linear_output` | %, × 100 | 0 % to 100 % | 1 % | 2 | 18 % | Always | Yes, with its picker ([below](#the-pickers)) |
| | Contrast | `curve_contrast_around_pivot` | × 1 | 0.1 to 10 | 0.05 | 2 | 3 | Always | Yes |
| | Shoulder power | `curve_shoulder_power` | × 1 | 0 to 10 | 0.05 | 2 | 3.3 | Always | Yes |
| | Toe power | `curve_toe_power` | × 1 | 0 to 10 | 0.05 | 2 | 1.5 | Always | Yes |
| *Show curve*, a collapsible group | The curve, read-only | — | | | | | | Always | Drawn disabled, to come, until a read-only graph row exists ([Gaps](#gaps), 1) |
| *Advanced curve parameters*, a collapsible group | Shoulder start | `curve_linear_ratio_above_pivot` | %, × 100 | 0 % to 100 % | 1 % | 2 | 0 % | Always | Yes |
| | Target white | `curve_target_display_white_ratio` | %, × 100 | 20 % to 100 % | 0.5 % | 2 | 100 % | Always | Yes |
| | Toe start | `curve_linear_ratio_below_pivot` | %, × 100 | 0 % to 100 % | 1 % | 2 | 0 % | Always | Yes |
| | Target black | `curve_target_display_black_ratio` | %, × 100 | 0 % to 15 % | 0.01 % | 2 | 0 % | Always | Yes |
| | Keep the pivot on the diagonal | `auto_gamma` | a toggle | | | | Off | Always | Yes |
| | Curve y gamma | `curve_gamma` | × 1 | 0.01 to 100 | 0.05 | 2 | 2.2 | While Keep the pivot on the diagonal is off | Yes |
| *Look*, a collapsible group | Slope | `look_slope` | × 1 | 0 to 10 | 0.01 | 2 | 1 | Always | Yes |
| | Lift | `look_lift` | × 1 | −1 to 1 | 0.005 | 2 | 0 | Always | Yes |
| | Brightness | `look_brightness` | × 1 | 0 to 100 | 0.01 | 2 | 1 | Always | Yes |
| | Saturation | `look_saturation` | %, × 100 | 0 % to 1000 % | 1 % | 2 | 100 % | Always | Yes |
| | Preserve hue | `look_original_hue_mix_ratio` | %, × 100 | 0 % to 100 % | 1 % | 2 | 60 % | Always | Yes |

**Primaries:**

| Group | Control | Parameter | Shown as | Range | Step | Decimals | Default | Shows | Ported |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| — | Base primaries | `base_primaries` | a list: Export profile, Working profile, Rec2020, Display P3, Adobe RGB (compatible), sRGB | | | | Rec2020 | Always | Every option but Export profile |
| | Disable adjustments | `disable_primaries_adjustments` | a toggle | | | | Off | Always | Yes |
| | Reset primaries | — (a line: its label and a menu of three actions) | | | | | | While adjustments are on | Yes ([below](#the-actions)) |
| *Before tone mapping*, a label | Red, green and blue attenuation | `red_inset`, `green_inset`, `blue_inset` | %, × 100, unfilled | 0 % to 99 % | 1 % | 2 | 29.46 %, 25.86 %, 14.64 % | While adjustments are on | Yes |
| | Red, green and blue rotation | `red_rotation`, `green_rotation`, `blue_rotation` | °, × 180/π, unfilled | ±30.0° | 0.1° | 1 | +2.0°, −1.2°, −3.6° | While adjustments are on | Yes |
| *After tone mapping*, a label | Reverse all | `completely_reverse_primaries` | a toggle | | | | Off | While adjustments are on | Yes |
| | Set from above | — (an action at Reverse all's end) | | | | | | While adjustments are on; unavailable while Reverse all is on | Yes ([below](#the-actions)) |
| | Master purity boost | `master_outset_ratio` | %, × 100 | 0 % to 200 % | 1 % | 2 | 100 % | While adjustments are on and Reverse all is off | Yes |
| | Master rotation reversal | `master_unrotation_ratio` | %, × 100 | 0 % to 200 % | 1 % | 2 | 0 % | The same | Yes |
| | Red, green and blue purity boost | `red_outset`, `green_outset`, `blue_outset` | %, × 100, unfilled | 0 % to 99 % | 1 % | 2 | 29.08 %, 26.32 %, 4.58 % | The same | Yes |
| | Red, green and blue reverse rotation | `red_unrotation`, `green_unrotation`, `blue_unrotation` | °, × 180/π, unfilled | ±30.0° | 0.1° | 1 | The rotations' | The same | Yes |

Each primary's attenuation and its rotation alternate, red, green then blue, as darktable's page has them, and so do the purity boosts and the reverse rotations. darktable draws those twelve sliders without their fill bar (`dt_bauhaus_slider_set_feedback(…, 0)`), and so do these rows.

### The pivot

darktable's pivot slider shows `curve_pivot_x`, the pivot's place in the range from 0 to 1, as an exposure. Its factor is the range's width and its offset its black, so it reads from black to white relative exposure, 0 EV at the default range. When the range moves, darktable's `_update_pivot_x` rewrites `curve_pivot_x` so the pivot keeps its exposure, inside the new range.

The tool stores the pivot in EV, as the slider shows it. Its binding writes `curve_pivot_x` from it and the range whenever any of the three is off its default: `(clamp(pivot, black, white) − black) / (white − black)`, in f32 as darktable computes both. So moving the range keeps the pivot's exposure ([A5](#invariants)).

Its slider is darktable's for the range (`ProcessingTool::range`, decision 30):

- It holds black to white relative exposure, and its track the same.
- It steps by darktable's step for the range's width (`slider_step`): 0.1 EV at the default 16.5 EV, 0.5 EV over the widest 40, 0.01 below 3.16.
- The definition's range, −20 to +20 EV, is the widest any range gives; its track and step are those at the defaults.

A pivot that a move of the range leaves outside is clamped to the range's nearer end and stored, kept, in the same change, as darktable's `_update_pivot_x` stores it. So lowering black past the pivot and raising it again leaves the pivot where the clamp put it, as in darktable. A pivot outside the range is refused ([A7](#invariants)). The binding still keeps the pivot inside the range, for any value an edit holds.

### Dynamic range scaling

darktable's safety margin for its pickers and Read exposure, which scale the range they compute by it. It is a control, as darktable shows it, over its whole range, −50 % to +200 %, stored in the edit as darktable stores it ([A8](#invariants)).

- **No render reads it,** so the operation has no parameter for it ([../ports/agx.md](../ports/agx.md), Parameters) and the binding sets none: moving it changes no parameter, no cache key and no frame.
- **The pickers and Read exposure read it** from the tool's values: each exposure the `white`, `black` and `range` pickers or Read exposure write is scaled by 1 + its value; the pivot pickers don't scale ([below](#the-pickers), [The actions](#the-actions)).
- **Moving it writes nothing into the range.** darktable's `gui_changed` rescales black and white relative exposure by `(1 + new) / (1 + previous)` as it moves, a write from one quantity into another that depends on the value it had, which rule 13 and decision 30 leave out ([Deliberate differences](#deliberate-differences)).

### The pickers

darktable's five, each an area picker, denoised, reading the operation's input, working RGB, linear Rec2020 (`DT_COLOR_PICKER_AREA | DT_COLOR_PICKER_DENOISE`), and `Agx::pick` writing what darktable's `color_picker_apply` writes (`agx.c:2657-2690`), each pick one step. `s` is Dynamic range scaling over 100 and ε darktable's `_epsilon`, 10⁻⁶.

| Picker | Where | Writes |
| --- | --- | --- |
| `white` | The quad on White relative exposure, "Picks the white point." | `log2(max(ε, m) / 0.18) · (1 + s)` of `m` the area's largest maximum of red, green and blue, held to 0.1 to 20 EV (`_apply_auto_white_exposure`) |
| `black` | The quad on Black relative exposure, "Picks the black point." | The same of the area's smallest minimum, held to −20 to −0.1 EV (`_apply_auto_black_exposure`) |
| `range` | Auto tune levels, darktable's labelled line with its picker at the end, a line row of pickers (`Row::Line`) | Both (`_apply_auto_tune_exposure`) |
| `pivot` | The quad on Pivot relative exposure | The pivot at `log2(max(ε, Y) / 0.18)` of the mean's luminance through the working profile, held within black and white relative exposure (`_apply_auto_pivot_x`) |
| `pivot_output` | The quad on Pivot target output | The pivot as `pivot` sets it, and Pivot target output the curve made from the values before the pick, with the old pivot, gives at the new pivot's place, raised to the curve's gamma, so the curve's linear output there, before the look, stays what it was; with Keep the pivot on the diagonal on the gamma moves with the pivot, so a look off its defaults moves the area (`_apply_auto_pivot_xy`) |

- **After `white`, `black` or `range`** the pivot keeps its exposure, held within the new range, as `_update_pivot_x` keeps it. The tool stores the pivot in EV, so the pick writes it only where the range's move clamps it.
- **The curve** for Pivot target output is `apertrust-color`'s `agx_curve`, the one the operation renders, made from the tool's values as the binding maps them: the pivot's place in the range, the percentages over 100, the gamma computed where Keep the pivot on the diagonal is on. The power is the curve's gamma too: Curve y gamma, or while Keep the pivot on the diagonal is on the gamma the curve computes, which darktable's `gui_changed` keeps in `p->curve_gamma` (`agx.c:1798-1804`) and this tool, never writing Curve y gamma, takes from the curve.
- **Curve y gamma is never written** by a pick, where darktable's writes the gamma it computes while Keep the pivot on the diagonal is on (`:2677-2685`; [Deliberate differences](#deliberate-differences); decision 63).
- **Each value is held** to its control's range by the session, and one that is not finite is left out. darktable's own bounds above already keep the exposures inside.

### The actions

darktable's three buttons, each an `Action` end item that writes controls, one change and one history step, and is never stored (decision 107):

- **Read exposure,** darktable's camera button at the end of Auto tune levels' line, after its picker: `_adjust_relative_exposure_from_exposure_params` (`agx.c:1042-1052`). It reads Exposure's published `effective_exposure` `e`, the exposure shown less what the operation takes off it, the stored one in Automatic mode too, and nothing else of the edit, and with `s` Dynamic range scaling sets black `(−8 + 0.5 e) · (1 + s)` held to −20 to −0.1 EV, in f32, and white `(4 + 0.8 e) · (1 + s)` held to 0.1 to 20 EV, in f64 as darktable's `0.8` is a double. The pivot keeps its exposure, clamped into the new range, as a pick keeps it; Curve y gamma is never written (decision 63). With no effective exposure read it writes nothing.
- **Reset primaries,** darktable's labelled line with its menu button (`agx.c:2280-2288`): a menu of Blender-like, Smooth and Unmodified (`:2100-2121`), each its own action, `primaries_blender_like`, `primaries_smooth` and `primaries_unmodified`, writing darktable's `_set_blenderlike_primaries`, `_set_smooth_primaries` and `_set_unmodified_primaries` (`:317-397`): adjustments on, Reverse all off, the base primaries, Rec2020 or for Smooth the working profile, and every primaries control, in the controls' units. Each binds to darktable's parameter to the bit where a value of its control reaches it, else one f32 step from it, A4's bound.
- **Set from above,** darktable's text button before Reverse all's switch (`agx.c:2339-2356`): `_set_post_curve_primaries_from_pre_callback` (`:2123-2140`), both masters to 100 %, each purity boost its attenuation and each reverse rotation its rotation, as shown, each its parameter's copy to the bit. Unavailable while Reverse all or Disable adjustments is on, darktable's `post_curve_primaries_available` (`:1744-1746`): drawn disabled at once as Reverse all turns on, and its registry action rests.

### The curve's warnings

Toe power and Shoulder power show darktable's warning sign before their values while the curve has lost its S shape and the fallback takes over, darktable's `need_convex_toe` and `need_concave_shoulder` (`agx.c:903`, `:948`, `:1727-1735`), from the curve the operation renders (`Curve::new`, its segments' `fallback`). The tooltip, and what screen readers hear first, is darktable's quad tooltip (`:1856-1880`): why, and what to change. The sign's place is kept while it is hidden, so the value never moves. darktable's preference to hide them is not offered (decision 107).

### What is not ported

- **The base primaries' Export profile** is listed, drawn disabled and refused, as the operation returns `NotPorted` for it until AgX reads the image context's output profile (decision 26).

The promises, the tooltips, are darktable's in British English: "Relative exposure above mid-grey (white point)", "Contrast in highlights. Higher values keep the slope nearly constant for longer, at the cost of a more sudden drop near white", and the like. Where a short label is ambiguous, its full name says which group it is in: Look slope, Curve contrast. A script sends `SetControl { tool: "agx", control: "curve_pivot_x", value: 1.0, done: true }`, the pivot at +1 EV over mid-grey.

The Ukrainian and Polish words are darktable's translations, drafts awaiting native review like every locale's. Two Polish ones need it most:

- "Ustal z góry" for Set from above, darktable's, which reads as "decide in advance".
- "Potęga zaokrąglenia" for Shoulder power, which beside the toe's "Potęga dolnego zaokrąglenia" lacks its "górnego".

The Polish file departs from darktable's po twice, as its header says: a typo fixed in Blue purity boost, and "(hue)" dropped from Preserve hue.

**Operation parameters in an edit.** The tool owns every `agx` parameter, so `apertrust-cli --param` refuses them. An edit that holds one as an operation parameter loses it with a warning when read ("the tool agx owns it now; left out") and stays editable.

**Presets:** darktable's, by its names: unmodified base primaries, blender-like base and punchy, sigmoid-like default and smooth, smooth; the scene-referred default is the tool's defaults. They wait for darktable's built-in presets, which the presets menu gains in its phase 2 (decision 106; [Gaps](#gaps), 7).

### Sliders against darktable

Audited against darktable's `src/iop/agx.c`, its params and `gui_init`, and against its panel as darktable draws it (the Settings tab's top: white relative exposure 6.50 EV, black −10.00 EV, dynamic range scaling +10.00 %, pivot relative exposure +0.00 EV, pivot target output 18.00 %, contrast 3.00, shoulder power 3.30, toe power 1.50; [README.md](README.md), Sliders against darktable).

`its_sliders_are_darktable_s`, in `crates/tools/src/agx.rs`, holds the definition to this table at the defaults, and `the_pivot_s_slider_is_darktable_s_at_every_input_range` the pivot's to darktable's at four other input ranges, from −20 to +20 EV down to −1 to +2 EV.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| White relative exposure | 0.1 to 20; 1 to 10 | EV, × 1 | 2, from params | 0.1 EV | No |
| Black relative exposure | −20 to −0.1; −20 to −1 | EV, × 1 | 2, from params | 0.1 EV | No |
| Dynamic range scaling | −0.5 to 2; soft maximum 0.5 | %, × 100 | 2, `gui_init` | 0.5 % | No |
| Pivot relative exposure | 0 to 1; none, shown from black to white | EV, × the range's width, + its black | 2, `gui_init` | darktable's for the width: 0.1 EV at the default 16.5 EV, 0.5 over 40, 0.05 over 5, 0.01 over 3, 0.1 over 12 | No: the range, track and step follow the input range through `ProcessingTool::range` (decision 30) |
| Pivot target output | 0 to 1; none | %, × 100 | 2, `gui_init` | 1 % | No |
| Contrast | 0.1 to 10; 0.1 to 5 | × 1 | 2, from params | 0.05 | No |
| Shoulder power, toe power | 0 to 10; 1 to 5 | × 1 | 2, from params | 0.05 | No |
| Shoulder start, toe start | 0 to 1; none | %, × 100 | 2, `gui_init` | 1 % | No |
| Target white | 0.2 to 1; 0.5 to 1 | %, × 100 | 2, `gui_init` | 0.5 % | No |
| Target black | 0 to 0.15; 0 to 0.025 | %, × 100 | 2, `gui_init` | 0.01 % | No |
| Curve y gamma | 0.01 to 100; 1 to 5 | × 1 | 2, from params | 0.05 | No |
| Slope | 0 to 10; 0 to 2 | × 1 | 2, from params | 0.01 | No |
| Lift | −1 to 1; −0.5 to 0.5 | × 1 | 2, `gui_init` | 0.005 | No |
| Brightness | 0 to 100; 0 to 2 | × 1 | 2, from params | 0.01 | No |
| Saturation | 0 to 10; 0 to 2 | %, × 100 | 2, `gui_init` | 1 % | No |
| Preserve hue | 0 to 1; none | %, × 100 | 2, `gui_init` | 1 % | No |
| Attenuations, purity boosts | 0 to 0.99; none | %, × 100 | 2, `gui_init` | 1 % | No |
| Rotations, reverse rotations | ±0.5236; none | °, × `RAD_2_DEG` | 1, `gui_init` | 0.1° | No |
| Master purity boost, master rotation reversal | 0 to 2; none | %, × 100 | 2, `gui_init` | 1 % | No |

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since every photo runs AgX; then the pivot's coordinates, the one control whose binding computes its parameter, what is not ported, the pivot's range following the input range (decision 30), Dynamic range scaling, which binds nothing, and the pickers.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| A0 | None: every control at its default, written or kept there | | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | The small raw through the export and the viewport; the ramp and the sweep; every corpus raw |
| A1 | Each moved, then reset | White relative exposure 8 EV; the pivot +1 EV; contrast 2 with toe power 2.5; saturation 140 % with lift 0.05; base primaries sRGB; Disable adjustments; black −8 EV with Keep the pivot on the diagonal, red rotation 10°, Reverse all and master rotation reversal 50 %; each reset by `ResetControl` and by `ResetTool` | The frame | Each move changes the frame; each reset renders as no edit | Exact | The sweep through the export; the small raw through the viewport; the 90D through the viewport and the export |
| A2 | Order | White relative exposure and the pivot; slope and black relative exposure; base primaries and green attenuation, each set first | The frame | The same, whichever was set first | Exact | The sweep; the 90D through the viewport and the export |
| A3 | On a layer | | The definition | Background only until layers come: the background's tone mapper's parameters are global; with layers a layer's own tone mapper replaces it within the layer's mask, never stacked (decision 33) | Exact | CPU |
| A4 | The darktable match | Every control set to darktable's value for each golden case, as darktable's slider shows it, unrounded | The parameters bound; the kernel's uniforms from `prepare`; the frame of the sweep through `agx` | darktable's parameters bit for bit, but those the binding's `value / factor` reaches from no f32 value of their control, one f32 step off, each reached by darktable's slider, which rounds to its digits; the kernel's uniforms the golden harness's (`golden::ports::kernel_inputs`) bit for bit where every parameter is; the frame the frame of darktable's parameters set directly | Exact, or one f32 step for an unreachable parameter, its uniforms within 1e-6 relative and its frame within a ΔE2000 max of 0.001 | `agx` (test 0175), `agx-default` (0174, params version 4), `agx-highlights` (0186) and `agx-edges`; the sweep |
| A5 | The pivot in EV | Pivot, black and white at (0, −10, 8), (1, −6, 6.5), (−2, −12, 3) and (5, −10, 4) EV, each set as the session sets it, the last pivot clamped to white as it moves | `curve_pivot_x`; the frame | `(clamp(pivot, black, white) − black) / (white − black)`, darktable's `_update_pivot_x`: the frame the frame of that parameter set directly | Exact | The ramp |
| A6 | What is not ported | Base primaries Export profile | The edit | Refused as not ported, the edit as it was | Exact | CPU |
| A7 | The pivot's range follows the input range (decision 30) | The pivot at −5 EV, then black to −4 EV; the pivot to −5 EV; black back to −10 EV | The edit; `curve_pivot_x` | Black's move stores the pivot at −4 EV, kept, in the same change, binding `curve_pivot_x` 0; −5 EV refused as outside −4 to +6.5 EV; black's return leaves the pivot at −4 EV, as darktable's `_update_pivot_x` leaves it | Exact | CPU |
| A8 | Dynamic range scaling | 20 %, then reset | The parameters bound; the plan's cache keys; the frame; the edit and its XMP | No parameter set, the keys and the frame as with no edit, black, white and the pivot unwritten; the edit holds 20 % and reads it back from its XMP; the reset leaves no edit | Exact | The ramp |
| A9 | The pickers | White over the ramp's brightest EV; black over its darkest; Pivot target output over a flat grey, the ramp's mid-grey and its 2⁻⁴ | The image entering AgX, read as the picker's area; the stored values; the frame | White `log2(max / 0.18) · 1.1` of the area's largest channel, black the same of its smallest, at the default scaling of +10 %; the grey's output in the frame where it was before the pick | 1e-4 EV and 1e-4 output, proposed | The ramp |

A4 is how the tool matches darktable. An untouched tool binds nothing, so the operation runs its defaults, darktable's scene-referred ones, which the operation's tests hold to test 0186's stored parameters field for field. A tool set to a golden case's values renders those parameters, which `apertrust-golden run` checks against darktable-cli's render.

A control stores the value its slider shows (README, Sliders against darktable), and the binding gives the parameter back as `value / factor`, in f32. For a few parameters that division steps over darktable's f32: no f32 percentage gives 0.6613 over 100, only 0.66129994 or 0.66130006.

darktable reaches them because its slider rounds a value set through it, dragged or typed, to its digits. `_slider_set_normalized` (`src/bauhaus/bauhaus.c`) stores `roundf(base · x) / base`, `base` 10^digits × its factor, then places it in the slider's range and reads it back, so 66.13 % is stored as 0.6613. A4 restates that path and holds it to the three values, so the one-step gap is the binding's, not darktable's arithmetic. Planned before the end of Phase 3, for every tool: each binding rounds a value as darktable's slider does, `roundf(base · x) / base`, so stored parameters are darktable's bit for bit, and the rows, A4's among them, are updated.

## Results

Measured on the Apple M4 (Metal) by `crates/tools/tests/agx.rs`, the tool's source and the panel's test. The synthetic and CPU rows run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| A0 | Holds, exactly. `bind` sets nothing with every control written at its default, or kept there. Frames byte-identical to no edit on the small raw through the export and the viewport and on the ramp and the sweep. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| A1 | Holds, exactly, for all seven moves on the sweep and the small raw, and on the 90D through the viewport and the export |
| A2 | Holds, exactly, for the three pairs on the sweep and on the 90D through the viewport and the export |
| A3 | Holds: the definition works on the background only, with no Auto |
| A4 | Holds: `agx-default`, `agx-highlights` and `agx-edges` darktable's to the bit; `agx` 27 of 35 parameters to the bit and three one f32 step off, ΔE2000 max 1.4e-4 on the sweep (below) |
| A5 | Holds, exactly, at all four settings, the last pivot clamped to white and stored there |
| A6 | Holds, exactly: refused as not ported, the edit unchanged |
| A7 | Holds, exactly |
| A8 | Holds, exactly |
| A9 | Holds: white 8.6276 EV within 2.1e-7 EV of `log2(max / 0.18) · 1.1` and black −8.8774 EV within 2.7e-7, from the image entering AgX; Pivot target output's grey within 1.4e-5 of its output before the pick, at both greys |
| By hand | To be filled by the maintainer: darktable 5.6.2's GUI on the 7R V and the X-T5, each of the five pickers on the same areas in both, the values each writes recorded here beside Apertrust's |

The hand check, to fill the last row: open the 7R V and the X-T5 in darktable 5.6.2 and in Apertrust, AgX at its defaults in both; pick the same areas with each picker, a bright one for White relative exposure, a dark one for Black, one area for Auto tune levels, a mid-tone for Pivot relative exposure and Pivot target output, from the defaults each time; note White and Black relative exposure, Pivot relative exposure and Pivot target output each writes. darktable's denoised area, the photo's size and the area's exact place differ, so values agree to a few hundredths of an EV, not to the bit.

The pickers' unit tests in `crates/tools/src/agx.rs` restate darktable's arithmetic in f64: black and white within 1.4e-6 EV of the bound of 1e-5 at Dynamic range scaling −50 %, 0, +10 % and +200 %; the pivot within 7.7e-8 EV and Pivot target output within 4.9e-8 in its parameter, with Keep the pivot on the diagonal off and on, of the bound of 1e-5, Curve y gamma never written.

A4 in detail:

- **`agx-default` and `agx-highlights`:** every parameter at its default, nothing bound. Every parameter, the kernel's uniforms and the sweep's frame are darktable's to the bit.
- **`agx-edges`:** 10 of 35 parameters off their defaults, every one darktable's to the bit, the uniforms and the frame too.
- **`agx`:** 30 of 35 off their defaults, 27 to the bit. Three are one f32 step off: red attenuation 66.13 % binds 0.66129994 for darktable's 0.6613, red purity boost 33.08 % 0.33079997 for 0.3308, and blue reverse rotation −4.40° −0.07679447 for −0.076794475. For each, the binding's `value / factor` reaches it from no f32 within 64 steps of the value shown, which brackets it, while darktable's slider path, rounding to its digits, reaches it from the value shown. The kernel's uniforms are at most 5.3e-7 from the case's, relative. On the sweep, against darktable's parameters set directly, ΔE2000 mean 5.0e-6, max 1.4e-4.
- **`apertrust-golden run`** on the four cases: ΔE2000 mean 0.0000 and max 0.0003 on `agx`, `agx-default` and `agx-highlights`, 0.0005 on `agx-edges`. Median passes of 3.5 to 4.2 ms at 10 MP and 7.1 to 7.9 ms at 20 MP over four runs, the operation's own.

The sliders: none differs from darktable's, the pivot's at the defaults and at four other input ranges ([Sliders against darktable](#sliders-against-darktable)).

## In the panel

- **Two tabs,** Settings and Primaries. On Settings, the Input exposure range label, its three sliders, and Auto tune levels' line, its picker and then Read exposure's camera at its end, in that Tab order (`tab_meets_auto_tune_levels_then_read_exposure`); the Basic curve parameters label and its five sliders, Toe and Shoulder power with their warnings (`toe_power_s_warning_follows_a_pending_change`, `agx-settings-line-dark`, `-light`); then the three collapsible groups, closed at first: Show curve, drawn disabled, its tooltip saying it is to come, as a tab with no rows; Advanced curve parameters, the gamma hidden while Keep the pivot on the diagonal is on; Look.
- **Dynamic range scaling** is live, at +10.00 % by default.
- **The pickers:** a quad on White and Black relative exposure, Pivot relative exposure and Pivot target output, and Auto tune levels' button, each named "…: pick from the photo", its tooltip darktable's words; a press sends `On` with the keys held (`agx_draws_its_five_pickers_and_a_press_sends_on_with_the_keys`). The Settings tab is photographed with Auto tune levels' picker on, Read exposure's camera after it (`agx-settings-pickers-dark`, `-light`).
- **On Primaries,** the base primaries list, Export profile drawn disabled, and Disable adjustments; while adjustments are on, Reset primaries' line with its menu button (`reset_primaries_opens_its_menu_and_smooth_sends_its_writes`), the Before tone mapping label and its six sliders, the After tone mapping label, Reverse all with Set from above after its switch, and while Reverse all is off the eight sliders after it (`agx_draws_darktable_s_page_its_actions_disabled`, `agx-primaries-dark`, `-light`, `agx-primaries-menu-dark`).
- **While closed** the section shows its title and buttons; its presets button opens its presets ([gui.md](../gui.md#a-tools-presets)), and its options hold Reset, Copy Settings and Paste Settings, then the layout's items.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **Show curve's graph.** darktable draws the tone mapping curve, read-only, with its pivot, toe and shoulder, in the Show curve section. A definition has no read-only graph row yet, so the group is drawn disabled and closed, its tooltip saying it is to come. Planned, as proposed: a custom widget of the design system's registry drawing `ProcessingTool::curve` for no control, with the operation's curve restated, as the Tone curve's `LumaCurve` is, held to the kernel by a row.
2. **Dynamic range scaling's write into the range.** Moving it in darktable rescales black and white relative exposure by `(1 + new) / (1 + previous)`, keeping the pivot's exposure: a write between two quantities that depends on the value it replaces, so the order controls were set in would matter, which rule 13 and decision 30 leave out. Here it is the pickers' and Read exposure's margin only, darktable's computation of the range, with no write into the range when it moves, so the order controls were set in never matters (principle 5): a deliberate difference ([below](#deliberate-differences)).
3. **Keep the pivot on the diagonal's write.** While it is on, darktable's `gui_changed` and `color_picker_apply` write Curve y gamma, hidden, to the gamma it computes, so turning it off leaves that gamma; here Curve y gamma keeps the value it had, after a move and after a pick, a deliberate difference ([below](#deliberate-differences)).
4. **The actions,** built: Read exposure on Auto tune levels' line, Reset primaries' menu and Set from above at Reverse all's end, as darktable has them ([The actions](#the-actions)).
5. **The primaries' painted tracks,** darktable's gradients of each primary's hue and purity, wait for the renderer, as RGB primaries' do.
6. **The curve's warnings,** built: a `Warning` end item on Toe power and Shoulder power, with no preference to hide them ([The curve's warnings](#the-curves-warnings)).
7. **darktable's presets** wait for the presets menu's built-in presets, its phase 2 (decision 106).
8. **A slider's own reset.** darktable's attenuation, rotation, purity boost and reverse rotation sliders reset to 0 on a double-click, and its two master sliders to 100 %, while its tool reset gives the scene-referred defaults; a control has one default here, the operation's (rule 4), which both a double-click and the tool's reset give.
9. **The tool's tooltip,** darktable's "applies a tone mapping curve. inspired by Blender's AgX tone mapper": a definition implies no message for the tool's own tooltip.

darktable's optional curve tab and its preference to show Look always open are preferences off by default (rule 12): the curve's sections stay on Settings and Look is a collapsible group.

## Deliberate differences

- **A slider's own reset** ([Gaps](#gaps), 8): the tool's default, where darktable's primaries sliders reset to 0 and its masters to 100 %.
- **Curve y gamma under Keep the pivot on the diagonal** ([Gaps](#gaps), 3): it keeps the value it had, after a move and after a pick, where darktable's `gui_changed` and `color_picker_apply` write it with the gamma they compute from the pivot and the range, a write between controls of different quantities, which rule 13 refuses since they must commute (decision 63).
- **Dynamic range scaling writes nothing into the range** ([Gaps](#gaps), 2): it is the pickers' and Read exposure's margin only, where darktable's rescales black and white relative exposure as it moves.
- **The curve's warnings always shown** ([The curve's warnings](#the-curves-warnings); decision 107): no preference hides them, where darktable's `plugins/darkroom/agx/enable_curve_warnings` can. Their tooltips keep darktable's words, but for its last line, which sends people to Show curve's graph, not built yet ([Gaps](#gaps), 1), and the toe's cause, black relative exposure where darktable's says white.
- **Reset primaries' three items are actions of their own** (decision 107): `tool-agx-primaries_blender_like`, `-primaries_smooth` and `-primaries_unmodified`, each reachable by a shortcut, where darktable defines only the button, `reset primaries`, whose menu a shortcut opens.

## Design choices

- **Control ids:** the operation's parameter names, as darktable's history names them, `curve_pivot_x` for the pivot in EV among them.
- **The pivot's range:** darktable's, the input range, its track the same and its step darktable's for the width, through `ProcessingTool::range`. A pivot a range's move leaves outside is clamped and stored (decision 30).
- **Dynamic range scaling:** a control binding no parameter, which only the pickers and Read exposure read. darktable stores it in the edit, so it is no setting, and the operation has no parameter for it; darktable's write into the range is refused by rule 13 ([Gaps](#gaps), 2).
- **The pickers:** darktable's five, each writing what its `color_picker_apply` writes; the pivot stored in EV, so a pick of the range writes it only where the new range clamps it; Pivot target output from `apertrust-color`'s curve, the operation's own, so a grey picked keeps its brightness in the frame with the look at its defaults ([A9](#invariants)).
- **Auto tune levels:** one line row (`Row::Line`), its label, then the `range` picker and Read exposure, an icon `Action` reading `Published { tool: "exposure", id: "effective_exposure" }`, as darktable's line.
- **The three actions:** `Action` end items where darktable's lines have them: Read exposure's camera (`camera`, derived from Lucide's `camera`), Reset primaries a line of its label and a menu button (`ActionEnd::menu`, `primaries-menu`, derived from Lucide's `list-restart`), and Set from above a text button at the end of Reverse all's toggle row. Each runs `Agx::action` on the values shown; Set from above's availability is `Agx::available`.
- **The warnings:** `End::Warning("toe")` and `End::Warning("shoulder")` at the two powers' ends, `Agx::warning` the curve's own `fallback`, so the sign shows exactly when the operation renders the fallback.
- **Show curve:** a collapsible group with no rows, which `ProcessingDefinition::check` takes, drawn disabled and closed with a titled tab's "to come" tooltip until a read-only graph row exists (`CollapsibleGroup::disabled`).
- **A4's bound:** bit for bit wherever the binding's `value / factor` reaches darktable's parameter from some value of the control. One f32 step for the three it reaches from none, each proved so and each reached by darktable's slider path, with the frame they render measured. Bindings that round as darktable's slider does are planned before the end of Phase 3 ([Invariants](#invariants)).
- **Rows:** A0 to A9. No first step, since every photo runs AgX; a row for the pivot's coordinates, one for what is not ported, one for the pivot's range following the input range, one for Dynamic range scaling and one for the pickers.
- **In `TOOLS`:** after Raw black/white point, Highlight reconstruction and Demosaic, and before Output colour profile. The Default workspace's Adjust tab lists it after Exposure (decision 38, `TAB_TOOLS`).
