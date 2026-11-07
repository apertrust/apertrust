# Demosaic

Module `demosaic`, version 1. It works on the background only and has no Auto.

It is darktable's demosaic in the first pass (decision 2). It has darktable's page, which has no tabs or presets and one collapsible section: the controls in `gui_init`'s order under darktable's labels in British spelling, each row shown where darktable's `gui_changed` shows it (decision 29).

It drives one operation, `demosaic` ([../ports/demosaic.md](../ports/demosaic.md)), which every raw runs, and owns all twelve of its parameters. Its defaults are the operation's for the photo ([README.md](README.md), rule 4), so an untouched module sets nothing.

**Status:** built, in `crates/modules/src/demosaic.rs`. The app draws it in the Base tab from its definition (`docs/gui.md`, Module panels from definitions), after Exposure and before Highlight reconstruction, White balance and Raw black/white point: [README.md](README.md) orders the tab as darktable does, the last in the pipeline at the top (decision 38).

- Its rows are tests in `crates/modules/tests/demosaic.rs` and the module's source, and the panel's in `crates/app/src/modules.rs`; [Results](#results) gives what they measured.
- A choice marked proposed is open; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

Each control is one `demosaic` parameter, its id the parameter's name. Each is shown as it is but sharp centre, shown in percent as darktable's "%" shows it.

Only RCD and Markesteijn 1-pass are ported, with colour smoothing and match greens disabled (`docs/ports/demosaic.md`, Deliberate differences). So:

- Every other option is listed, drawn disabled and refused (rule 6).
- The controls that only the other methods and capture sharpening read are not ported at all (`Control::unported`): each takes only its default, the operation's, and is drawn disabled.

| Control | Parameter | Shown as | Range | Step | Decimals | Default | Shows | Ported |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Method (`demosaicing_method`) | `demosaicing_method` | a list of the sensor's methods, of darktable's 17 (below) | | | | The photo's, darktable's `reload_defaults`: RCD for a Bayer sensor, Markesteijn 1-pass for an X-Trans one, VNG4 for a Bayer4 one, Monochrome for a monochrome one | Always | RCD and Markesteijn 1-pass |
| Dual threshold (`dual_thrs`) | `dual_thrs` | a number | 0 to 1 | 0.01 | 2 | 0.2 | The dual methods | No |
| Edge threshold (`median_thrs`) | `median_thrs` | a number | 0 to 1 | 0.01 | 3 | 0 | PPG | No |
| LMMSE refine (`lmmse_refine`) | `lmmse_refine` | a list: Basic, Median, 3x median, Refine & medians, 2x refine + medians | | | | Median | LMMSE | No |
| Colour smoothing (`color_smoothing`) | `color_smoothing` | a list: Disabled, Once to Five times | | | | Disabled | Every method but the passthroughs, the dual methods and a monochrome sensor's | Disabled only |
| Match greens (`green_eq`) | `green_eq` | a list: Disabled, Local average, Full average, Full and local average | | | | Disabled | A Bayer sensor's methods but its passthroughs | Disabled only |
| Capture sharpen (`cs_enabled`) | `cs_enabled` | a toggle | off or on | | | Off | Every method but the passthroughs | No |
| *Capture sharpen controls*, a collapsible group | | | | | | | While Capture sharpen is on | |
| Iterations (`cs_iter`) | `cs_iter` | a whole number | 1 to 25 | 1 | 0 | 8 | | No |
| Radius (`cs_radius`) | `cs_radius` | px | 0 to 1.5 px | 0.01 px | 2 | 0 px | | No |
| Contrast sensitivity (`cs_thrs`) | `cs_thrs` | a number | 0 to 1 | 0.01 | 2 | 0.4, the operation's | | No |
| Corner boost (`cs_boost`) | `cs_boost` | px | 0 to 1.5 px | 0.01 px | 2 | 0 px | | No |
| Sharp centre (`cs_center`) | `cs_center` | %, × 100 | 0 % to 100 % | 1 % | 0 | 0 % | darktable's: while Corner boost is off 0 ([Gaps](#gaps)) | No |

### The method

The method lists darktable's methods in the order its lists give them:

| Sensor | Its methods |
| --- | --- |
| Bayer, nine | PPG, AMaZE, VNG4, RCD, LMMSE, RCD (dual), AMaZE (dual), Passthrough (monochrome), Photosite colour (debug) |
| X-Trans, seven | VNG, Markesteijn 1-pass, Markesteijn 3-pass, Frequency domain chroma, Markesteijn 3-pass (dual), Passthrough (monochrome), Photosite colour (debug) |
| Bayer4, three | VNG4 and the two Bayer passthroughs |
| Monochrome, one | Monochrome |

Each option's id is the operation's choice of that name, X-Trans's passthroughs `passthrough_monochrome_xtrans` and `photosite_color_xtrans`, and the binding writes the operation's choice by its name. The definition holds all seventeen, so an edit means the same on every photo. The method lists the sensor's alone, as darktable's four comboboxes do, one shown per sensor (`Module::options`, decision 30).

The method is live, its unported methods drawn disabled, as Exposure's Mode lists its automatic option. On a monochrome sensor, listing only the method it holds, it is unavailable (decision 30), though the engine renders no such raw. A raw with no colour filters is taken as darktable's monochrome sraw; a linear raw has none either, and the image context cannot yet tell the two apart (`docs/ports/demosaic.md`, Open questions).

**Another sensor's method** is refused as a command (`EditError::NotListed`). It reaches an edit only from that sensor's photo, pasted or by hand. There it is kept and rendered as the photo's own, as darktable's `commit_params` falls back ([D1](#invariants)), and shown last in the list, chosen and disabled, "Not available for this photo", until another method is chosen. Its rows follow it as stored ([Gaps](#gaps), 2).

### The rows

**The rows follow the method,** as darktable's `gui_changed` shows them (`Row::When`), the method saying the sensor:

- dual threshold for the three dual methods;
- edge threshold for PPG;
- LMMSE refine for LMMSE;
- colour smoothing for every method but the four passthroughs, the three dual ones and Monochrome;
- match greens for a Bayer sensor's methods but its passthroughs;
- capture sharpen for every method but the passthroughs, and its group while it is on.

So a Bayer raw shows Method, Colour smoothing, Match greens and Capture sharpen, and an X-Trans raw Method, Colour smoothing and Capture sharpen, as darktable's panel does ([D5](#invariants)). Capture sharpen takes only off, so its group never shows. On an image that is not a raw, where darktable's page says demosaic does not apply, every control is unavailable.

The promises, the tooltips, are darktable's in British English: "How many colour smoothing median steps after demosaicing", "Green channels matching method. Only applies for some old sensors", "Capture sharpen recovers details lost due to in-camera blurring…", and the like. The method's are its two, for Bayer and X-Trans sensors, in one ([Gaps](#gaps), 3). A script sends `SetControl { module: "demosaic", control: "demosaicing_method", value: …, done: true }`, which takes the sensor's ported method and refuses every unported one and every other sensor's.

**Presets:** darktable's demosaic has none.

### Sliders against darktable

Audited against darktable's `src/iop/demosaic.c`, its params and `gui_init`, and against its panel as darktable draws it for a Fujifilm X-Trans raw: method Markesteijn 1-pass, color smoothing disabled, the capture sharpen toggle ([README.md](README.md), Sliders against darktable). `its_sliders_are_darktable_s`, in `crates/modules/src/demosaic.rs`, holds the definition to this table. None of these rows shows while its method or capture sharpening is not ported.

| Slider | darktable: range, soft range | Shown as | Digits | darktable's step, the control's | Differs |
| --- | --- | --- | --- | --- | --- |
| Dual threshold | 0 to 1; none | × 1 | 2, `gui_init` | 0.01 | No |
| Edge threshold | 0 to 1; none | × 1 | 3, `gui_init` | 0.01 | No |
| Iterations | 1 to 25, an int; none | × 1 | 0, an int | 1 | No |
| Radius, corner boost | 0 to 1.5; none | px, × 1 | 2, `gui_init` | 0.01 | No |
| Contrast sensitivity | 0 to 1; none | × 1 | 2, from params | 0.01 | No |
| Sharp centre | 0 to 1; none | %, × 100 | 0, `gui_init` | 1 % | No |

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since demosaic runs on every raw, and order, since the method is the only control that takes a value off its default; then what is not ported and the rows darktable shows. No control's unit is Apertrust's but sharp centre's percent.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| D0 | None: every control at its default, written at the photo's or kept there | | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | The small Bayer raw and a small X-Trans raw through the export and the viewport; every corpus raw |
| D1 | The method, moved and reset | The other sensor's ported method: Markesteijn 1-pass on a Bayer raw, RCD on an X-Trans one, set on a photo of that sensor, as an edit from it holds it; reset by `ResetControl` and by `ResetModule` | Whether this photo takes it as a command; the method the binding writes; the plan's operations and cache keys; the frame | Refused here (`NotListed`) and kept when the edit holds it; the other method written; the operation falls back to the photo's own, as darktable's `commit_params` does, so the frame and its keys are no edit's, and so once reset | Exact | The small raws through the export and the viewport; the 90D and the X-T5 through the viewport and the export |
| D2 | On a layer | | The definition | Background only: never offered on a layer, as demosaic acts on the mosaic | Exact | CPU |
| D3 | The darktable match | None | The method an unedited photo, and each untouched edit, renders | darktable's `reload_defaults`, restated: RCD (5) on a Bayer sensor, Markesteijn 1-pass (1025) on an X-Trans one, VNG4 on a Bayer4 one and Monochrome on a monochrome one in the unit tests, every other parameter darktable's default but contrast sensitivity ([Deliberate differences](#deliberate-differences)); `apertrust-golden raw-run` holds the operation at those parameters to darktable's demosaic run with the same method | Exact | The small raws; every corpus raw |
| D4 | What is not ported | Every method but RCD and Markesteijn 1-pass; colour smoothing and match greens but Disabled; LMMSE refine Basic; capture sharpen on; dual and edge threshold and capture sharpening's sliders at their ranges' ends | The edit | Each command refused as not ported, the edit as it was | Exact | CPU |
| D5 | The rows and lists darktable shows | Each of the seventeen methods, capture sharpen off and on; each sensor: Bayer, X-Trans, Bayer4, monochrome and an image that is not a raw | The rows shown; the method's list (`Module::options`); `available` | darktable's `gui_changed` restated: each row shown exactly where darktable shows it, the capture group only while capture sharpen is on and never for a passthrough; darktable's `gui_init` restated: each sensor's methods in its combobox's order, holding the photo's default, every method listed for some sensor; the method unavailable only on a monochrome sensor, where it lists one; every control unavailable on an image that is not a raw | Exact | CPU; the panel, for a Bayer raw and an X-Trans raw |

D3 is how the module matches darktable. An untouched module binds nothing, so the operation runs its defaults, darktable's `reload_defaults` for the sensor as the test restates it. `raw-run` checks the operation at those parameters against darktable-cli demosaicing with the method Apertrust gives it.

Neither asks darktable for its own default method, so the restatement is what D3 holds to. `raw-run` measures a mean difference of 8e-9 to 8e-7 on the Bayer raws and 2e-8 to 3e-7 on the X-Trans ones ([../ports/demosaic.md](../ports/demosaic.md), Results). No golden case moves a demosaic parameter, since no other method is ported.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/demosaic.rs` and the module's source. The small-raw and CPU rows run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| D0 | Holds, exactly. `bind` sets nothing with every control written at the photo's default, or kept there. Frames byte-identical to no edit on the small Bayer and X-Trans raws through the export and the viewport. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| D1 | Holds, exactly, on the small raws through the export and the viewport, and on the 90D and the X-T5 through the viewport and the export. The other sensor's method is refused as a command and, from that sensor's photo, kept and written; the operation prepares the very passes it prepares for no edit, so even the cache keys are no edit's. The resets render as no edit |
| D2 | Holds: the definition works on the background only, with no Auto |
| D3 | Holds on the small raws and on all 31 corpus raws: 27 Bayer, RCD, and 4 X-Trans, Markesteijn 1-pass. `raw-run` agrees (below) |
| D4 | Holds, exactly: each command refused as not ported, the edit unchanged |
| D5 | Holds for all seventeen methods with capture sharpen off and on, and for each sensor's list (`d5_each_sensor_lists_darktable_s_methods`); the panel too (below) |

D3 in detail: `apertrust-golden raw-run`, on darktable's test raws and on the 29 corpus raws darktable loads, gives the Bayer raws that pass within 8.2e-9 to 9.0e-8 of darktable on average, and the X-Trans ones within 2.7e-8 to 1.4e-7. The two lossy Sony ARWs, `hlrecovery.arw` and `mire1.cr2` fail, as they do without the module.

D5 in the panel (`demosaic_draws_darktable_s_rows_for_the_photo_s_method`):

- **A Bayer raw:** Method (RCD, live, its list PPG disabled and no VNG), Colour smoothing and Match greens (Disabled, live) and Capture sharpen (off, disabled).
- **An X-Trans raw:** the same with Markesteijn 1-pass, its list VNG disabled and no PPG, and no Match greens, the capture group hidden.
- **A Bayer raw holding Markesteijn 1-pass from an X-Trans raw's edit:** that method last in its list, chosen and disabled, the box described "Not available for this photo", its rows following it.

## In the panel

- **No tabs:** darktable's controls one under another in its order, only those its `gui_changed` shows for the method; capture sharpening's five in the Capture sharpen controls group, shown while capture sharpen is on, which it cannot be until capture sharpening is ported.
- **The method** is a live list of the sensor's methods, RCD or Markesteijn 1-pass chosen, the unported drawn disabled; another sensor's method an edit holds comes last, chosen and disabled, "Not available for this photo".
- **Colour smoothing and Match greens** are lists, live, Disabled the one option ported, the others listed disabled, as Vignetting's Dithering is.
- **Capture sharpen** is a toggle row drawn disabled, off.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **A Bayer4 sensor's rows.** The method lists a Bayer4 sensor's three methods (decision 30), but rows follow the method alone, and VNG4 is a Bayer sensor's method too, so a Bayer4 raw shows colour smoothing, match greens and capture sharpen, which darktable hides there. The engine renders no Bayer4 raw.
2. **Another sensor's stored method's rows.** An edit from another sensor's photo may hold its method, which the method shows last, chosen and disabled, and refuses as a command (decision 30). Its rows follow it as stored: Match greens is hidden on a Bayer raw holding Markesteijn 1-pass. darktable's `gui_changed` (`demosaic.c`, lines 1556 to 1579) maps it to the sensor's method, shows that one's rows and writes it back. Decision 30 leaves this open.
3. **The method's tooltip,** darktable's per sensor, for Bayer and X-Trans: one promise holds both.
4. **Sharp centre's condition.** darktable shows it only while corner boost is off 0; rows follow a choice or a toggle, so it stays in the group, which shows only while capture sharpen is on, not ported.
5. **The quads:** dual threshold's, contrast sensitivity's and corner boost's mask views (showmask), and radius's quad that computes it from the sensor's data, come with their tools; their rows never show while their methods and capture sharpening are not ported.
6. **Contrast sensitivity's default** ([Deliberate differences](#deliberate-differences)).
7. **The page for an image that is not a raw:** darktable shows "not applicable" in place of the controls; the panel draws them disabled.
8. **The module's tooltip,** darktable's "reconstruct full RGB pixels from a sensor color filter array reading": a definition implies no message for the module's own tooltip.

## Deliberate differences

Where the module's defaults are not darktable's, following the operation's (`docs/ports/demosaic.md`, Deliberate differences):

- **Contrast sensitivity** starts at 0.4, the params' default. darktable's `reload_defaults` computes `cs_thrs` from the photo (`_get_variance_threshold`, `src/iop/demosaicing/capture.c`, line 49): 0.07 lower for a raw white point above 4096, then 0.012 lower for each 100 ISO below 600 and higher above, the ISO taken within 100 to 1000.

  Capture sharpening is not ported, so the value never renders, and it stays the operation's until that port.

## Design choices

- **Control ids:** the operation's parameter names, `demosaicing_method`, `color_smoothing`, `green_eq` and `cs_center` among them, so `--set` and scripts name what darktable's history names, as Vignetting's do.
- **Option ids:** the operation's choices by name, but LMMSE refine's three that start with a digit, `median_3x`, `refine_medians` and `refine_2x_medians`, each at the operation's place.
- **The method's list:** the sensor's methods, as darktable's comboboxes list them (decision 30), the method live and unavailable only on a monochrome sensor. Each list holds its default:
  - A raw with no colour filters is taken as darktable's monochrome sraw, so the operation's default for it is darktable's Monochrome, though a linear raw has none either and takes RCD in darktable (`docs/ports/demosaic.md`, Open questions).
  - A raw of four colours takes darktable's VNG4, whatever its pattern's size.
- **What is not ported:** the method's and the two lists' options not ported are listed and refused, as Vignetting's Dithering. Dual and edge threshold, LMMSE refine and capture sharpening's six controls are not ported at all, taking only their defaults, as Exposure's deflicker controls, since no ported method reads them.
- **Defaults:** the operation's for the photo: darktable's `reload_defaults` for the method, the operation's 0.4 for contrast sensitivity ([Deliberate differences](#deliberate-differences)).
- **Rows:** D0 to D5. No first step, since every raw runs demosaic, and no order row, since only the method can move.
- **The panel's test:** in `crates/app/src/modules.rs`, beside every other module's.
- **Row ids:** D, for Demosaic.
- **In `MODULES`:** after Raw black/white point and Highlight reconstruction, and before AgX.
