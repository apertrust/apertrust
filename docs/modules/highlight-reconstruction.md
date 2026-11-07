# Highlight reconstruction

Module `highlights`, version 1. It works on the background only, as the whole raw stage does, and has no Auto.

It is darktable's highlight reconstruction in the first pass (decision 2). darktable's page has no tabs, sections or presets of its own: every control is in `gui_init`'s order under darktable's labels, each shown for the methods darktable shows it for (decision 29). It is in the Base tab, after Rotate and perspective and before White balance, as darktable's base group lists it (decision 38; [README.md](README.md), Highlight reconstruction).

It drives one operation, `highlights` (raw 4, [../ports/highlights.md](../ports/highlights.md)), which every raw runs, with inpaint opposed its one method ported. So it adds no operation: it has no first step, and at its defaults it sets nothing and leaves the plan as it was. It owns every parameter of the operation, the method and the clipping threshold. A change runs demosaic again, as every raw-stage control does (decision 12; [../engine.md](../engine.md), Where the viewport's scale applies).

**Status:** built, in `crates/modules/src/highlight_reconstruction.rs`, in `MODULES` after Raw black/white point. The app draws it in the Base tab after Demosaic and before White balance (decision 38), from its definition (`docs/gui.md`, Module panels from definitions).

- Its rows are tests in `crates/modules/tests/highlight_reconstruction.rs`; [Results](#results) gives what they measured.
- Every row is exact or darktable's own match, so none has a proposed tolerance.
- The [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

Each control is darktable's parameter as its slider shows it ([Sliders against darktable](#sliders-against-darktable)):

- darktable's hard range is the control's, in the units shown: a percentage is the parameter times 100 where darktable's `"%"` format multiplies a factor whose hard maximum is within ±10.
- There is no soft range, as darktable sets none.
- The digits are darktable's, and the step is what `dt_bauhaus_slider_get_step` gives over the range.

Control ids are darktable's parameter names, and the method's options the operation's choices.

| Control | Parameter | Range or options | Step | Default | Shows |
| --- | --- | --- | --- | --- | --- |
| Method (`mode`) | `mode` | Inpaint opposed (`opposed`); Reconstruct in LCh (`lch`), Clip highlights (`clip`), Segmentation based (`segments`), Guided laplacians (`laplacian`), not ported | — | Inpaint opposed | Always |
| Clipping threshold (`clip`) | `clip` | 0 to 2, three decimals | 0.01 | The photo's: the lower of 1 and its linear response limit | Always |
| Combine (`combine`) | none yet | 0 to 8, no decimals; not ported | 0.1 | 2 | Segmentation based |
| Candidating (`candidating`) | none yet | 0 % to 100 %, no decimals; not ported | 1 % | 40 % | Segmentation based |
| Rebuild (`recovery`) | none yet | Off, Generic, Flat generic, Small segments, Large segments, Flat small segments, Flat large segments; not ported | — | Off | Segmentation based |
| Strength (`strength`) | none yet | 0 % to 100 %, no decimals; not ported | 1 % | 0 % | Segmentation based, Rebuild not off |
| Noise level (`noise_level`) | none yet | 0 to 0.5, three decimals; not ported | 0.005 | 0 | Guided laplacians; segmentation based with Rebuild not off |
| Iterations (`iterations`) | none yet | 1 to 256, a whole number; not ported | 1 | 30 | Guided laplacians |
| Inpaint a flat colour (`solid_color`) | none yet | 0 % to 100 %, no decimals; not ported | 1 % | 0 % | Guided laplacians |
| Diameter of reconstruction (`scales`) | none yet | 2 px to 4096 px, darktable's twelve (128 px and 256 px "slow", 512 px and 1024 px "very slow", 2048 px and 4096 px "insanely slow"); not ported | — | 128 px (slow) | Guided laplacians |

**Method** lists darktable's methods for a Bayer raw, as its `reload_defaults` builds the list: from inpaint opposed to guided laplacians in its enum's declaration order. Inpaint opposed is ported and the default; the others are drawn disabled and refused as values (rule 8 of [README.md](README.md)).

darktable's list depends on the sensor:

| Photo | darktable's methods |
| --- | --- |
| Bayer raw | All five |
| X-Trans raw | Inpaint opposed, reconstruct in LCh, clip highlights and segmentation based |
| Linear raw | Inpaint opposed and clip highlights |
| Not a raw, or a four-colour sensor's raw | Clip highlights alone, the default there too |

darktable's enum's last value, reconstruct colour, is listed for no sensor, so the module lists it nowhere.

**For now** the module lists darktable's Bayer list on every photo. So an X-Trans raw shows guided laplacians too, disabled as on a Bayer raw. Method stays enabled on a colour raw, as darktable's combobox is, though it has one option to choose, as Exposure's Mode does.

**Planned:** the module lists each photo's own through `Module::options` (decision 30: a choice lists the options darktable lists for the photo). Then:

- **each photo lists darktable's methods,** as in the table above;
- **Method defaults to clip highlights** where darktable's `reload_defaults` does, an image that is not a raw or a four-colour sensor's raw, and to inpaint opposed elsewhere;
- **a stored method the photo does not list** binds as darktable's `gui_changed` rewrites it: clip highlights on an image that is not a raw or a four-colour sensor's raw; inpaint opposed for guided laplacians on a raw that is not Bayer, and for reconstruct in LCh or segmentation based on a linear raw;
- **Method is unavailable** where the photo lists one option only (decision 30).

`available` reads a raw with no colour filter array (`cfa: None`) as monochrome, which darktable shows "not applicable" for. A linear raw has none either, and lists two methods, so the image context must tell the two apart before the lists come.

**Clipping threshold** is where clipping starts, as a fraction of each colour's white point. Inpaint opposed counts a photosite as clipped at 0.987 of it, times its colour's white balance coefficient (`docs/ports/highlights.md`, Setup).

Its default is the operation's for the photo (rule 4, decision 21), darktable's `reload_defaults`: the lower of 1 and the raw's linear response limit, a DNG's LinearResponseLimit, 1 when the raw records none, at least 0. The module cannot call the operation's `defaults`, so it restates it from the image context (`threshold`). Its unit tests hold the two together on limits below, at and above 1, below 0 and not finite, and K0 on every corpus raw. The binding sets `clip` only while the control is off it, as it is.

A script sends `SetControl { module: "highlights", control: "clip", value: 0.95 }`, and `apertrust-cli edit --set highlights.clip=0.95` does the same. `--param highlights.clip=0.95` is refused, since the module owns it.

**The other methods' controls**, segmentation based's and guided laplacians', are darktable's page too (rule 3). Their rows follow the method, and within segmentation based the rebuild mode, as darktable's `gui_changed` shows its widgets; Noise level has a row under each method that shows it.

Their methods are not ported, so these controls are not either (`Control::unported`). Each takes only darktable's default, a value refused otherwise, and their rows never show, since their methods cannot be chosen. The operation's schema holds only the method and the threshold, which inpaint opposed reads. The other parameters come with their methods (decision 51), and the module owns them then.

**On a photo that is not a colour raw** every control is unavailable, drawn disabled: an image that is not a raw runs no raw stage, and on a monochrome raw darktable shows a "not applicable" page in place of the controls (`reload_defaults`, `gui_init`), the operation refusing it.

**Not shown:** darktable's showmask buttons beside Clipping threshold, Combine, Candidating and Strength. They show the clipped highlights or the segments in false colours and store nothing, and wait for the renderer's quad ([Gaps](#gaps)).

**Presets:** darktable's `highlights.c` has none.

The words are darktable's labels and tooltips from its `en@truecase` catalogue, in British English ("colour", "favour"), a tooltip's lines joined into sentences, with a full stop. Ukrainian and Polish are from darktable's `uk.po` and `pl.po` at the tag, capitalised as the other modules' are.

### Sliders against darktable

Audited against darktable's `src/iop/highlights.c`, its params and `gui_init`, and against darktable's panel as it draws it for a raw: "method" at "inpaint opposed" and "clipping threshold" at "1.000", the track at its middle, with the showmask button ([README.md](README.md), Sliders against darktable). darktable's side is in its parameters' units, with the factor its slider shows them by.

`its_sliders_are_darktable_s`, in `crates/modules/src/highlight_reconstruction.rs`, holds the definition to this table.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Clipping threshold | 0 to 2; none | as it is | 3 (`dt_bauhaus_slider_set_digits`) | 0.01 | No |
| Combine | 0 to 8; none | as it is | 0 (`dt_bauhaus_slider_set_digits`) | 0.1 | No |
| Candidating | 0 to 1; none | "%", × 100 (`dt_bauhaus_slider_set_format`) | 0 (`dt_bauhaus_slider_set_digits`) | 1 % | No |
| Strength | 0 to 1; none | "%", × 100 | 0 (`dt_bauhaus_slider_set_digits`) | 1 % | No |
| Noise level | 0 to 0.5; none | as it is | 3 (`dt_bauhaus_slider_from_params`) | 0.005 | No |
| Iterations | 1 to 256 (an `int`); none | as it is | 0 | 1 | No |
| Inpaint a flat colour | 0 to 1; none | "%", × 100 | 0 (`"%"` takes two of `dt_bauhaus_slider_from_params`' two) | 1 % | No |

The steps:

- **The threshold's** range, 2, is under 100, so darktable takes a hundredth of it, 0.02, to the power of ten below it, 0.01, as 0.02 lies less than half a decade above.
- **Combine's** 8 gives 0.08, so 0.1 from the power just above it, within a tenth of a decade. Its row shows no decimals, so a key moves it by at least 1, darktable's `min_visible`.
- **A percentage's** range of 1 shows as 100, so 1 %.
- **Noise level's** 0.5 gives 0.005, half a decade above 0.001, so 5 times it.

No slider differs.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since the module adds no operation, and the layer row, since it works on the background only.

- Only Clipping threshold can move: the method takes only inpaint opposed and the other controls only their defaults, which K0 and the unit tests hold.
- No control's unit is Apertrust's: the threshold is darktable's parameter as it is, and K3 holds it to darktable's parameters.
- The module acts on the raw stage, so every row renders from a mosaic: the small raw's, which clips in a corner, or a corpus raw's.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| K0 | None: every control at its default for the photo, written at it or kept there | | The module's defaults against the operation's for the photo; `bind`; the plan's instances and cache keys; the frame, on one pipeline that the threshold one step off its default passes through first | Every default is the operation's; `bind` returns nothing; instances, keys and frames as with no edit | Exact | The small raw through the export and the viewport; every corpus raw (defaults, instances and keys) |
| K1 | Clipping threshold, moved and reset | From the photo's default: one step down and one up, down by 0.5, where far more photosites count as clipped, and up by 1, to 2, where none does; each reset by `ResetControl` and by the header's reset (`ResetModule`) | The plan's operations and cache keys; the frame | Each move changes the frame; each reset as K0 | Exact | The small raw through the export and the viewport; 90D through the viewport and the export |
| K2 | Order | Clipping threshold down 0.05 with Raw black/white point's White point down 100, either first; and the threshold set through a value 0.3 above first | Frame | The same whichever came first, and whatever the threshold held before | Exact | The small raw through the export; 90D through the viewport and the export |
| K3 | Clipping threshold, the darktable match | 0, 0.5, 0.9, 0.987, 1, 1.3 and 2, on a photo with no linear response limit and one whose limit is 0.9 | The default; the parameters the edit renders; the history entry `apertrust-golden raw-run` writes for them; the kernel's clipping levels through the operation's `prepare` | darktable's default, parameters and clipping levels, bit for bit (below) | Exact | The values, on the CPU; every corpus raw, for the default |

K3 must hold:

- **The default** is darktable's `reload_defaults` threshold, the lower of 1 and the limit, on those photos and on every corpus raw.
- **The edit** renders inpaint opposed, darktable's value 5, and the threshold bit for bit, at darktable's offsets in its highlights v4 entry.
- **Each colour's clipping level** is darktable's `_process_opposed` one, `0.987f × clip` times its coefficient, bit for bit.

K3 is how the module matches darktable. `apertrust-golden raw-run` writes the parameters an edit renders into darktable's history and holds the raw stage to darktable's render with them: 29 of 33 mosaics pass, the other four failing for their decoders or camera matrix (`docs/ports/highlights.md`, Results). K3 holds the module's threshold, at its default and moved, to give exactly darktable's parameters and clipping levels.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/highlight_reconstruction.rs`. The small raw's rows and K3's values run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| K0 | Holds, exactly (below) |
| K1 | Holds, exactly, on the small raw through the export and the viewport, and on the 90D through the viewport and the export. Every move changes the frame, each one step among them, and resets render as no edit |
| K2 | Holds, exactly, on the small raw and on the 90D through the viewport and the export |
| K3 | Holds, exactly, for every value on both photos (below) |

- **K0:** every default is the operation's, on synthetic contexts with limits below, at and above 1, below 0 and not finite, and on all 31 corpus raws the pipeline takes. `bind` sets nothing with every control written at its default or kept there. Frames are byte-identical to no edit on the small raw through the export and the viewport, each on one pipeline that a moved threshold passes through first. On all 31 corpus raws, the same instances and cache keys as no edit.
- **K3:** inpaint opposed and the threshold bit for bit in the edit's parameters and in darktable's highlights v4 entry, and darktable's clipping levels in the kernel's inputs. On all 31 corpus raws the default is 1, darktable's: none records a linear response limit below 1.

## In the panel

- **No tabs and no group labels:** Method and Clipping threshold, then, when their method is ported and chosen, its rows, in darktable's order.
- **Method** is a `Choice` list, inpaint opposed the one option live, the others drawn disabled.
- **Clipping threshold** shows three decimals, "1.000", in the language's number format; its track is plain over 0 to 2, filled from the photo's default. A key steps by 0.01, Shift by 0.1, Alt by 0.001, the last decimal shown; a value is rounded to three decimals before it is sent.
- **On a photo that is not a colour raw** every row is drawn disabled.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The showmask buttons** beside Clipping threshold, Combine, Candidating and Strength: the renderer draws no quad of that kind yet, and the false-colour views they toggle need the engine to render a mask. They store nothing, so no edit waits on them.
2. **The method's list per sensor,** planned through `Module::options` (decision 30). darktable leaves guided laplacians out of an X-Trans raw's list, lists two methods for a linear raw and clip highlights alone for an image that is not a raw or a four-colour sensor's raw. Until the module gives each photo its own list, it lists the Bayer list there too, every method but inpaint opposed disabled, as every method not ported is. What changes then is under [Controls](#controls): each photo's list, clip highlights as the default where darktable's is, `gui_changed`'s rewrites for a stored method the photo does not list, and Method unavailable where one option is listed. The image context has to tell a linear raw from a monochrome one first.
3. **The other methods,** and their controls with them, wait for their ports (decision 51): reconstruct in LCh, clip highlights, segmentation based and guided laplacians.
4. **The module's tooltip,** darktable's "avoid magenta highlights and try to recover highlights colors": a definition implies no message for the module's own tooltip, as for Grain.
5. **A drag's frame time.** A change restarts the viewport at the uploaded mosaic and runs highlight reconstruction's statistic and demosaic at full resolution, as Raw black/white point's do (`docs/modules/raw-black-white-point.md`, Gaps).

## Design choices

- **Module id and control ids:** `highlights`, darktable's operation name (rule 1), and darktable's parameter names. The method's options take the operation's choice ids; Rebuild's and the diameter's take darktable's descriptions as ids (`flat_generic`, `px_128`).
- **The method's options:** darktable's list for a Bayer raw, in its order, on every photo for now, until each photo lists darktable's own through `Module::options` (decision 30; [Controls](#controls)). Reconstruct colour is left out, as darktable lists it for no sensor.
- **The other methods' controls:** controls not ported, at darktable's defaults, in rows following their method as `gui_changed` shows them, owning no parameter until their methods come, as the `tool-bindings` skill's rule 6 allows.
- **Noise level:** one control in two rows, one under each method that shows it, never shown at once, with the control's own words, as the `tool-bindings` skill's rule 3 and `docs/engine.md`'s module layer allow. One row under a condition of both methods is something `Row::When` cannot say.
- **The default threshold:** the operation's for the photo, restated in the module from the image context, as Raw black/white point restates its levels, with tests holding the restatement to the operation's own.
- **Method's availability:** enabled on a colour raw, as darktable's combobox is, though only inpaint opposed can be chosen, as Exposure's Mode.
- **A photo that is not a colour raw:** every control unavailable, drawn disabled, where darktable shows "not applicable" for a monochrome raw and a list of clip highlights alone for an image that is not a raw.
- **The showmask buttons:** left out until the renderer draws such a quad.
- **Order in `MODULES`:** after Raw black/white point.
- **Words:** darktable's labels and tooltips from its `en@truecase` catalogue, a tooltip's lines joined, with British spelling ("colour", "favour", "power lines"); Ukrainian and Polish from its `uk.po` and `pl.po` at the tag, capitalised. The Polish draft corrects darktable's `pl.po` where it is wrong:
  - "wiąż się" to "wiąże się";
  - Combine's tooltip without its second sentence, about a mask button, which translates an older msgid;
  - the speed qualifiers `pl.po` drops on 256, 1024 and 2048 px restored, as on the other diameters;
  - "1024 pikseli" to "1024 piksele".
- **Row ids:** K, for the clipping it reconstructs.
- **The rows:** K0 to K3. No first step, as the module adds no operation, and no layer row, as it works on the background only; K1 and K3 for the threshold, the one control that can move.
