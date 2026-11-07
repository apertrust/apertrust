# Raw black/white point

Module `rawprepare`, version 1. It works on the background only, as the whole raw stage does, and has no Auto.

It is darktable's raw black/white point in the first pass (decision 2). darktable's page has no tabs, sections or presets of its own: its four black levels and its white point in `gui_init`'s order under darktable's labels, then Flat field correction (decision 29). darktable's scene-referred preset puts it in no group, so it takes Base, last, the group darktable's other layouts give it (decision 38; [README.md](README.md), In no group).

It drives one operation, `rawprepare` (raw 1, [../ports/rawprepare.md](../ports/rawprepare.md)), which every raw runs. So it adds no operation: it has no first step, and at its defaults it sets nothing and leaves the plan as it was. It owns every parameter of the operation. A change runs demosaic again, as every raw-stage control does (decision 12; [../engine.md](../engine.md), Where the viewport's scale applies).

**Status:** built, in `crates/modules/src/raw_black_white_point.rs`, first in `MODULES`. The app draws it in the Base tab, last, after White balance, from its definition (`docs/gui.md`, Module panels from definitions).

- Its rows are tests in `crates/modules/tests/raw_black_white_point.rs`; [Results](#results) gives what they measured.
- Every row is exact or darktable's own match, so none has a proposed tolerance.
- The [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

Each level is one `rawprepare` parameter as it is ([Sliders against darktable](#sliders-against-darktable)):

- a whole raw value, shown with no decimals, as darktable's sliders show its `uint16_t` fields;
- darktable's hard range, 0 to 65535, the control's;
- darktable's soft range, 0 to 16384, the span its track shows;
- darktable's step, 1, what `dt_bauhaus_slider_from_params` gives an integer.

Control ids are the operation's parameter names, darktable's with the array's index as a suffix.

| Control | Parameter | Range | Track's span | Step | Default |
| --- | --- | --- | --- | --- | --- |
| Black level 0 (`raw_black_level_separate_0`) | `raw_black_level_separate_0` | 0 to 65535 | 0 to 16384 | 1 | The raw's black at even rows and even columns of the sensor |
| Black level 1 (`raw_black_level_separate_1`) | `raw_black_level_separate_1` | 0 to 65535 | 0 to 16384 | 1 | At even rows and odd columns |
| Black level 2 (`raw_black_level_separate_2`) | `raw_black_level_separate_2` | 0 to 65535 | 0 to 16384 | 1 | At odd rows and even columns |
| Black level 3 (`raw_black_level_separate_3`) | `raw_black_level_separate_3` | 0 to 65535 | 0 to 16384 | 1 | At odd rows and odd columns |
| White point (`raw_white_point`) | `raw_white_point` | 0 to 65535 | 0 to 16384 | 1 | The raw's white point |
| Flat field correction (`flat_field`) | `flat_field` | Choice: Disabled (`off`), Embedded GainMap (`embedded`), not ported | — | — | Disabled |

A black level is the black of one position of the sensor's 2×2 pattern of row and column parities, row parity × 2 + column parity, on the sensor rather than the crop (`docs/ports/rawprepare.md`, Per pixel). The operation scales each photosite to `(raw − black) / (white − black)`.

A script sends `SetControl { module: "rawprepare", control: "raw_white_point", value: 16000 }`, and `apertrust-cli edit --set rawprepare.raw_white_point=16000` does the same. `--param rawprepare.raw_white_point=16000` is refused, since the module owns it, and `--param rawprepare.left=4` as fixed by the module, which gives the crop no control (below). An edit that holds `rawprepare` values at operation level, under `param:`, loses them with a warning when it is read, and they are gone from the file after its next save.

**Every default is the photo's** (rule 4 of [README.md](README.md), decision 21): the operation's own default for the photo, which is darktable's `reload_defaults` over rawler's levels (decision 23).

- Each black is the decoder's black pattern at the position's row and column, the pattern repeating across the 2×2 one, and every level is rounded to a whole number in 16 bits. The white point is the decoder's.
- On the Fujifilm X-T5 of the corpus that is 1020 for every black and 16383 for the white, as darktable's own panel shows them.
- The module cannot call the operation's `defaults`, so it restates them from the image context (`levels`). Its unit tests hold the two together on every kind of black pattern, and R0 on every corpus raw.
- An image that is not a raw runs no raw stage. Its levels are the operation's schema's, darktable's passthrough preset, black 0 and white 65535, and every control is unavailable, drawn disabled where darktable shows its page saying the module "only works for the sensors that need it".
- A level is bound only while it is off the photo's default, as the whole number nearest it, darktable's `uint16_t`. The panel sends whole numbers already.

**Flat field correction** lists darktable's two options. Embedded GainMap is not ported, since rawler reads no DNG opcode lists and so no gain maps (`docs/ports/rawprepare.md`, Deliberate differences). It is refused as a value, so Disabled, the default, is the only value the control takes, and it never sets anything. darktable shows the row only for a raw whose gain maps it can apply (`gui_update`, `_check_gain_maps`), which no photo is for Apertrust. So the control can change nothing on any photo and is drawn disabled on every one (decision 30: disabled, never hidden).

**Not shown** (rule 12): darktable's crop label and its left, top, right and bottom margins, which darktable shows only while its preference to edit the raw crop is on, off by default. The module owns the four parameters, at the raw's margins, the sensor's geometry, and never sets them, so no edit, script or `--param` moves them.

**Presets:** darktable's passthrough, black 0 and white 65535 with no crop, is left out ([README.md](README.md)). It disables the raw stage's scaling, which no edit needs, and it sets the crop, which the module leaves fixed.

**Not copied:** darktable flags the module `IOP_FLAGS_UNSAFE_COPY` (`rawprepare.c`, `flags`), so copying a history to other images leaves it out, a raw's levels being its own. Copy edit and Apply edit are planned to skip it the same way.

The words are darktable's labels and tooltips from its `en@truecase` catalogue, in British English, with a full stop. darktable's tooltip for each level is its label, "Black level 0." and "White point.", and Flat field correction's is "Raw flat field correction to compensate for lens shading.". Ukrainian and Polish are from darktable's `uk.po` and `pl.po` at the tag, capitalised as the other modules' are.

### Sliders against darktable

Audited against darktable's `src/iop/rawprepare.c`, its params and `gui_init`, and against darktable's panel as it draws them for a Fujifilm X-T5 raw: "black level 0" to "black level 3" at 1020 and "white point" at 16383, whole numbers ([README.md](README.md), Sliders against darktable).

`its_sliders_are_darktable_s`, in `crates/modules/src/raw_black_white_point.rs`, holds the definition to this table.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Black level 0 to 3 | 0 to 65535 (`uint16_t`, `$MAX: UINT16_MAX`); 0 to 16384 (`dt_bauhaus_slider_set_soft_max`) | as it is | 0 (`dt_bauhaus_slider_from_params` for an integer) | 1 | No |
| White point | the same | as it is | 0 | 1 | No |

No slider differs. The crop margins' four sliders, 0 to 65535 with a soft maximum of 256, are not shown.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since the module adds no operation, and the layer row, since it works on the background only.

- No control's unit is Apertrust's: each level is darktable's parameter as it is, and R3 holds the module's values for darktable's own levels to darktable's parameters.
- The module acts on the raw stage, so every row renders from a mosaic, the small raw's or a corpus raw's, never a synthetic image in the working space.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| R0 | None: every control at its default for the photo, written at it or kept there | | The module's defaults against the operation's for the photo; `bind`; the plan's instances and cache keys; the frame, on one pipeline that a level one step off its default passes through first | Every default is the operation's; `bind` returns nothing; instances, keys and frames as with no edit | Exact | The small raw through the export and the viewport; every corpus raw (defaults, instances and keys) |
| R1 | Each control, moved and reset | From the photo's defaults: Black level 0 and Black level 3 up one step, White point down one; every black up 16; the white point down by a twentieth; every level at once; each reset control by control (`ResetControl`) and by the header's reset (`ResetModule`) | The plan's operations and cache keys; the frame | Each move changes the frame; each reset as R0 | Exact | The small raw through the export and the viewport; 90D through the viewport and the export |
| R2 | Two controls, in either order | Black level 0 up 8 with White point down 100; Black level 3 up 4 with Black level 1 up 2 | Frame | The same whichever was set first | Exact | The small raw through the export; 90D through the viewport and the export |
| R3 | Every level, the darktable match | darktable's own levels for each raw on which `apertrust-golden raw-defaults` recovers a level of darktable's that is not Apertrust's: the 40D (`mire1.cr2`), the M50, the R6 Mark II, the Z 9, the OM-1 Mark II, the K-3 Mark III and the NX1, set as the module's controls over Apertrust's levels for the raw | The parameters the edit renders; the history entry `raw-run` would write for them (`golden::raw_stage::rawprepare_from`); the kernel's inputs through the operation's `prepare` | Apertrust's levels as defaults; darktable's history entry and `commit_params` bit for bit (below) | Exact | The levels, on the CPU; the seven raws, for the defaults |
| R4 | White point at or below a black level | White point at the black, 0 on the small raw; Black level 0 at 100 with White point at 100; Black level 0 at 100 and Black level 3 at 200 with White point at 50 | The raw stage's output, demosaic's, before the frame's clamps could hide a value | Every value finite: the operation keeps each divisor at one level (`docs/ports/rawprepare.md`, Deliberate differences), where darktable's is 0 or negative | Exact | The small raw through the export and the viewport |

R3 must hold:

- **The module's defaults** are Apertrust's levels for each raw, which the decoder gives (checked on the raws themselves).
- **The edit** renders darktable's levels with the raw's crop, so darktable's history entry for them bit for bit.
- **The kernel's inputs** are darktable's `commit_params`: each black, and the white point less it.

R3's darktable levels, the test's table and the one under Results, are transcribed from `apertrust-golden raw-defaults`' output, which recovers them from darktable's render. The test checks Apertrust's side against the decoder on the raws themselves, so a decoder that changes a level fails it, and a darktable release that changes one needs the table transcribed again.

R3 is how the module matches darktable. `apertrust-golden raw-run` holds the operation's kernel to darktable's render at the parameters it is given, within 7.2e-7 on 30 raws (`docs/ports/rawprepare.md`, Results). R3 holds the module's controls set to darktable's own levels to give exactly those parameters, so darktable's default levels for any raw are one edit away. Where the levels differ the defaults are rawler's, each difference with its verdict in the port's Results (decision 23). The Pixel 4a is left out of R3, since its difference is darktable's embedded gain maps, which no level reproduces.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/raw_black_white_point.rs`. The small raw's rows and R3's levels run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, and R3's with `APERTRUST_TEST_IMAGES` as well, `#[ignore]`d.

| Row | Result |
| --- | --- |
| R0 | Holds, exactly (below) |
| R1 | Holds, exactly, on the small raw through the export and the viewport, and on the 90D through the viewport and the export. Every move changes the frame, each level's one step among them, and resets render as no edit |
| R2 | Holds, exactly, on the small raw and on the 90D through the viewport and the export |
| R3 | Holds, exactly, on all seven raws: the module's defaults are the levels `raw-defaults` gives as Apertrust's, and darktable's own levels set as controls give darktable's history entry and `commit_params` bit for bit |
| R4 | Holds: every value finite, through the export and the viewport. Without the operation's guard the first case gives infinities on the export, so the row reads the raw stage, where the frame's clamps do not hide them |

**R0 in detail:** every default is the operation's, on synthetic contexts with every kind of black pattern and on all 31 corpus raws the pipeline takes. `bind` sets nothing with every control written at its default or kept there. Frames are byte-identical to no edit on the small raw through the export and the viewport, each on one pipeline that a moved level passes through first. On all 31 corpus raws, the same instances and cache keys as no edit.

**The golden harness's raw-stage checks.** `apertrust-golden raw-run` and `raw-defaults` plan the raw stage through the modules with no edit. They print the same lines with the module as without it, on darktable's five test raws and the 40 corpus samples:

- `raw-defaults` checks 33 raws, 5 passing, the rest differing by their crop or by the levels below, which R3 sets, and skips 12.
- `raw-run`, the raw stage and colorin, passes 29 of 33, as the port specs record, and skips 12.

darktable's levels, where they differ from Apertrust's:

| Raw | Black: darktable's / Apertrust's | White: darktable's / Apertrust's |
| --- | --- | --- |
| Canon EOS 40D (`mire1.cr2`) | 1030, 1030, 1026, 1026 / 1023 | 13600 / 13600 |
| Canon EOS M50 | 510, 510, 511, 511 / 510, 511, 511, 511 | 11892 / 11892 |
| Canon EOS R6 Mark II | 511 / 512 | 14008 / 12735 |
| Nikon Z 9 | 1008 / 1008 | 15892 / 15520 |
| OM System OM-1 Mark II | 255, 254, 254, 255 / the same | 4096 / 4095 |
| Pentax K-3 Mark III | 256 / 256 | 16315 / 16375 |
| Samsung NX1 | 128 / 0 | 16100 / 16100 |

## In the panel

- **No tabs and no group labels:** darktable's four black levels, its white point and flat field correction, one under another, in its order.
- **Values** show as whole numbers, darktable's digits, in the language's number format, the fill from the photo's own level.
- **Tracks** are plain, over darktable's soft range, 0 to 16384, widening to show a value beyond it, a 16-bit white point among them, as darktable's does.
- **Keyboard:** every level is a `SliderRow`, stepping by 1, Shift by 10; a value is rounded to a whole number before it is sent.
- **Flat field correction** is a `Choice` drawn disabled at Disabled on every photo. A disabled box does not open, so Embedded GainMap is not seen; once the control is available its list opens, an unported option there drawn disabled.
- **On an image that is not a raw** every row is drawn disabled.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **Monochrome and sRAW rows.** darktable hides Black level 1 to 3 for a monochrome sensor and Black level 3 for an sRAW, and its `gui_changed` writes the hidden ones from the others. The engine takes neither kind of raw yet (`docs/ports/rawprepare.md`, Deliberate differences), so every row shows for every photo it opens; they come with the raws.
2. **A white point at or below a black level.** Each control's range is darktable's, so one drag of the white point, or of a black, can put the white at a black or below it. At a black, darktable divides by zero, writing infinities and NaN into highlight reconstruction, demosaic and the scopes; below it, its divisor is negative and the photosites of that position invert. Neither the module nor darktable keeps the white above the blacks. The operation does instead: it keeps each divisor, white less black, at least one level, so such a photosite scales to its raw value less the black, finite, and everything else is darktable's bit for bit (`docs/ports/rawprepare.md`, Deliberate differences; row R4).
3. **The module's tooltip,** darktable's "sets technical specificities of the raw sensor": a definition implies no message for the module's own tooltip, as for Grain.
4. **The passthrough preset** is left out, by the mapping.
5. **A drag's frame time.** A change restarts the viewport at the uploaded mosaic and runs demosaic at full resolution, as a White balance drag does. `docs/engine.md` estimates it from the per-operation medians: about 40 ms at the 40D's 10 MP and 240 ms at the 7R V's 61 MP on the M4, before the viewport-size rest. A spike is planned to time it.

## Design choices

- **Module id:** `rawprepare`, darktable's operation name (rule 1).
- **Control ids:** the operation's parameter names, darktable's with the array's index as a suffix (`raw_black_level_separate_0`), so `--set` names what darktable's history names, as RGB primaries' and Grain's do.
- **The defaults:** the operation's for the photo, restated in the module from the image context, as Exposure restates the camera bias's clamp. Tests hold the restatement to the operation's own on every kind of black pattern and on the corpus. `Base`, which a binding has, is not available to `Module::defaults`.
- **A level as a number:** a number control stepping by 1 with no decimals, bound as the nearest whole number, as the Tone equalizer's Filter diffusion is. The panel sends whole numbers, and a fractional value from a script binds as darktable's slider rounds a typed one.
- **Flat field correction:** a choice of darktable's two options, Embedded GainMap not ported, and unavailable on every photo, since darktable shows it only for a raw with usable gain maps and rawler reads none. The control as a whole is not marked not ported (`Control::unported`), since its parameter's off value is ported.
- **The crop margins:** owned with no control, at the raw's values, so nothing sets them; `--param` is refused as for a parameter the module fixes (`Module::fixed`). Every operation parameter has exactly one owning module (AGENTS.md; `tool-bindings`).
- **An image that is not a raw:** every control unavailable, drawn disabled, where darktable shows a page with a sentence in place of the sliders.
- **Order in `MODULES`:** first, so the session reports it and an edit saves it first.
- **Words:** darktable's labels and tooltips from its `en@truecase` catalogue, the tooltips its labels where `gui_init` sets them so; Ukrainian and Polish from its `uk.po` and `pl.po` at the tag, capitalised. Polish keeps darktable's "Korekta płaskich obszarów" for the label and "korekcja płaskiego pola" in the tooltip.
- **Row ids:** R, for Raw black/white point.
- **The rows:** R0 to R4. No first step, as the module adds no operation, and no layer row, as it works on the background only; R4 for the operation's guard on a white point at or below a black level.
