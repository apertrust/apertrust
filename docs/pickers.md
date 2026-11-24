# Colour pickers

A colour picker reads the colour of a point or an area of the photo. A tool's picker turns what it reads into the tool's controls: Exposure's sets the exposure that maps the area to a target lightness, White balance's the coefficients that make it neutral. It has two doors, its button in the tool and its cursor in the Toolbar's well ([cursors.md](cursors.md)). The Colour picker shows what the photo holds under a point or an area, the live sample, and keeps numbered stored samples to compare. This document describes them as one API: the session's commands and events, how the engine measures on the GPU, how a tool declares its pickers and what it does with a reading, the picker button and the area on the photo, the Colour picker, and how the work is built. The session's other commands and events are in [session-protocol.md](session-protocol.md) and [events.md](events.md); history's steps in [history.md](history.md).

**Status.** [Decision 79](decisions/0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md) records the design, with the options considered. Built, 79.1 of [The plan](#the-plan): the Colour picker's own picker through the session, `Command::Picker` with `On`, `Point`, `Area`, `Switch`, `Reset` and `Off`, and `Event::Picker` with its frame part; its region kept in the original image's coordinates through the geometry; the reduction's `picker_rgba` and `picker_combine` on the frame; the staging pair the scopes share, `readback.rs`; `Region`, `Stats` and `PickKeys` in `apertrust-tools`. Built, 79.2: `apertrust-tools`'s side, [Declaring pickers](#declaring-pickers), `ProcessingDefinition::pickers` with their checks, `Quad::Picker(id)`, `Row::Pickers` and `ProcessingTool::pick`, `testing::Sample`'s three pickers, and Exposure's, Colour balance RGB's six and AgX's four slider pickers declared and named by their quads, their picks to come with each tool's pull request. Built, 79.3: the engine's side of tool pickers, the reduction at an operation's input and output in the live viewport pipe, `picker_mosaic` per CFA colour, denoise, buffers larger than one binding read in row windows, `On`'s capture with the operation as the edited instance, sample-only renders, a pick applied through `ProcessingTool::pick` as one change, a drag's one gesture, the tool's last reading, turning off, `Settings` and `Channel`, the pickers' buffers counted in the plan's memory, and the moved area's budgets measured on the M4. Built, 79.4: the design system's on-photo overlay primitives ([gui.md](gui.md#on-photo-overlays)). Built, 79.5: the design system's `PickerButton` and `PickerMarks` ([The picker button](#the-picker-button), [On the photo](#on-the-photo)), in the gallery. Built, 79.6: the app's side, driven by `testing::Sample`: the quads and rows of pickers drawn with their state, the marks over the photo, the viewer bar's words, the pointer's and the keys' tables, the keys taken before the app's actions, focus and the live region; a tool's pickers are drawn once its pick is built. Built, 79.7: Exposure's picker, the first a person can use: its pick (`Exposure::pick`) and its settings in the settings schema (`tools.exposure.*`, `apertrust-tools`'s `settings`), sent with `On` and `Settings`; it picks toward Picker target's lightness (decision 88), Middle grey, Lightness or a reference, its tooltip and its hint over the photo following Picker target (`Picker::words_by`), Lightness shown under Picker target while chosen (`Row::WhenSetting`), the reference row while Reference is, with the `reference` picker measuring an area's L across photos (`Row::Reference`), Exposure's picker waiting for one (`ProcessingTool::picker_ready`, `NotReady`), Last pick and its warnings under Picker target (`Row::LastPick`, `Shown::last` and `warnings`, `Reading::input_maximum`), and its page has no area mode, measure mode or patches. `Row::Patches` and the design system's `Patches` stay for Colour calibration's area mapping. Built, 79.8: White balance's picker, `spot`, turned on by Settings' from image area (`PresetRow::pickers`, an option that turns a picker on, its one place until the button bar comes), its pick darktable's `color_picker_apply` on the mosaic's means (`WhiteBalance::pick`), Settings reading from image area after a pick and, once the tool is redrawn, user modified, or as shot or camera reference where the picked coefficients match one within 1e-5, as the photo opens and after Undo, Redo or a `GoTo` (`ProcessingTool::redrawn`, `Edit::redraw`), a change not yet a step. Built, 79.9: Colour calibration's picker, `illuminant`, on the CAT tab's readout row, darktable's CCT text and the light's colour beside it (`Row::Pickers::readout`, `ProcessingTool::readout`, `ToolState::readouts`), its pick darktable's `_auto_set_illuminant` in both area modes (`ColourCalibration::pick`), and the area colour mapping live, its settings in the settings schema (`tools.channelmixerrgb.*`), the input's L, h and c under its patch, and its title "(active)" (`ProcessingTool::group_active`). Built, 79.10: Colour balance RGB's six pickers (`ColourBalanceRgb::pick`), darktable's `color_picker_apply`: the four hues' quads writing a wheel's hue and chroma from the area's Ych, the opposite hue unless `command` is held, and the white and contrast grey fulcrums', through `apertrust-color`'s `xyz_to_ych`. The rest is planned in Phase 2, in the pull requests of The plan: each other tool's pick, White balance's button bar, AgX's Auto tune levels with its row, stored samples, the other spaces, and the Colour picker tool; the rows darktable gives the pickers' settings are drawn disabled (`Row::Setting`, `Row::Action`). Built, 79.17: the engine's side of the one-shot pick, `PickerCommand::Once`, for scripts, the MCP and assistants, and `Region::pressed`, which the app's press on the photo makes its new area with ([A one-shot pick](#a-one-shot-pick)); the app sends it from nowhere. Built with decision 104's 104.5 ([cursors.md](cursors.md#the-plan)): the well showing the picker that is on, Esc wherever the focus is and A held over a picker, the crosshair, on and off said to screen readers with the reason, and the app's turn-offs for no focused tool; with 104.6, the picker cursor as the button's second door, sending the same `On`, White balance's, Exposure's two, Colour calibration's and Colour balance RGB's four hue pickers', a tool's advanced pickers, as Colour balance RGB's two fulcrums', having none; with 104.7, the label on the area and the pointer's badge ([On the photo](#on-the-photo)). Planned, [decision 105](decisions/0105-picker-gestures-a-click-within-6-points-the-pickers-own-kinds.md), 105.1 to 105.5: a press on the photo a click while within 6 points, the picker's declared kinds deciding what a click and a drag make, a click's area a fixed size on screen, no switch on the photo, and the session keeping a picker that takes one kind to it ([On the photo](#on-the-photo), [Commands](#commands)).

## What it serves

| Client | What it needs |
| --- | --- |
| A tool's pickers | Exposure, White balance, Colour calibration, Colour balance RGB, AgX, RGB curve and RGB levels in the first pass, then Filmic RGB and the Colour equalizer; each reads its operation's input, or its output too, and writes its controls as one step |
| The Colour picker | darktable's color picker utility: the colour under a point or an area of the photo as it is shown, the live sample and stored samples, statistics and colour modes, samples shown on the photo, the scopes restricted to the area |
| Pickers that only show | RGB curve's and blending's "show colour", the Colour equalizer's hue: drawn on the tool's graph, writing nothing |
| Later | Blending's pickers on a layer (Phase 4), in LCh, HSL or JzCzhz; Colour calibration's colour checker, its patches' means; the assistant and MCP, picking an area by its coordinates |

Every client of a session uses the same commands and receives the same readings. The app's mouse and keys are one way to send them; a script names a point in the photo's coordinates and gets the same values and the same step.

## darktable's pickers

The reference release's design, which Apertrust mirrors (decision 2):

- **One active picker** (`color_picker_proxy.c:54-58`): `darktable.lib->proxy.colorpicker.picker_proxy` is a module's picker or the color picker utility's, never two. Turning one on turns the one before it off (`:166-172`); a click on the active one turns it off (`:237-252`).
- **A picker's kinds** (`color_picker_proxy.h:34-44`): `DT_COLOR_PICKER_POINT`, `_AREA` or both; `_DENOISE`, a B-spline blur before sampling, 4-channel images only; `_IO`, the module's output measured as well as its input. A picker with both kinds turns on as an area with Ctrl held or the right button, else as a point (`color_picker_proxy.c:177-193`).
- **Its colour space,** `picker_cst` (`color_picker_proxy.h:51-56`): the module's own, LCh for Lab modules or HSL for RGB ones, set by blending; `dt_color_picker_new_with_cst` fixes another, JzCzhz for the colour equalizer and none for white balance, which then reads whatever the pipe holds there, the raw mosaic included (`temperature.c:2082-2090`).
- **Turning one on** turns its module on (`color_picker_proxy.c:174-175`), takes the picker's last point or area, or the default, the photo's centre or 2 % to 98 % of it (`:194-210`, `lib.c:1602-1612`), gives the module focus (`:225-229`) and forces the next reading to be applied (`:235`).
- **Where it measures** (`pixelpipe_hb.c:1182-1198`, `:1568-1588`): in the preview pipe, a downscaled whole image, so an area outside the visible part still measures; only for the focused module while it is enabled; its input and, for `_IO`, its output, each in `picker_cst`; then `DT_SIGNAL_CONTROL_PICKERDATA_READY`.
- **What it measures** (`color_picker.c:456-565`): per channel the mean, the minimum and the maximum over the box; on a mosaic, per CFA colour (`:277-305`); in a converted space, a fourth channel holding the third moved half a turn, so a hue round 0 has a minimum and a maximum (`_update_stats_4ch`, `:46-57`); denoised first with `blur_2D_Bspline` at one scale (`:476-502`).
- **The box** (`color_picker.c:394-454`): the point or area is kept in the original image's coordinates; its four corners are mapped forward to the module's input or output, sorted, the two lowest and the two highest averaged on each axis, at least one pixel, clamped to the image; one wholly outside it measures nothing (`:443-444`).
- **When a reading is applied** (`color_picker_proxy.c:288-328`): only when the point or area moved since the last, at the picker's start, or after its colour space changed (`_record_point_area`, `:76-104`). A reading after an edit elsewhere is measured and shown, never applied.
- **When a picker turns off:** another turned on; the module changed from its GUI (`dt_iop_gui_changed`, `imageop.c:3997-4006`) or redrawn (`gui_update`, as `exposure.c:683`), unless the picker is "keep-active" (`color_picker_proxy.c:127`), as RGB curve's show colour (`rgbcurve.c:1502`); the module losing focus (`imageop.c:2414-2416`); another image (`darkroom.c:1100-1102`). The utility's picker turns off on any focus change unless "restrict scope to selection" is on (`imageop.c:2393-2395`).
- **The color picker utility** (`libs/colorpicker.c`) samples the gamma module's input, the display-referred frame, for the active picker, its primary sample, and each of its "live samples", the stored samples here (`pixelpipe_hb.c:1113-1172`, `:3017-3027`): RGB in the display profile, converted to the histogram profile and to Lab.

## The model

```mermaid
flowchart LR
    subgraph session["Session (view state, never in the edit)"]
        A["The active picker<br/>a tool's, or the Colour picker's"]
        L["Stored samples<br/>each locked or not"]
        R["Each picker's last point and area"]
    end
    A -->|"tool picker"| M["Reduction at its operation's<br/>input and output, in the viewport pipe"]
    A --> F["Reduction on the frame"]
    L --> F
    M --> P["ProcessingTool::pick → one change, source Auto"]
    M --> E1["Event::Picker"]
    F --> E1
    F --> E2["Event::Samples"]
```

- **One active picker** per session, as darktable has it: a tool's, named by its tool and its id, or the Colour picker's (`PickerRef::ColourPicker`, darktable's primary picker with no module).
- **The live sample** is what the active picker measures on the frame, darktable's primary sample; **stored samples** are darktable's "live samples": areas or points copied from the active picker, numbered, each measured on every frame unless locked, any number, coexisting with the active picker. The interface uses the wireframe's words, live and stored; darktable's code names the second kind live samples.
- **Both are view state:** the session keeps them, as it keeps what the scopes count and, for the open photo alone, the eye's hidden tools; nothing is saved in the edit, and they stay across photos for the session's life, as darktable keeps its "live samples" across images. The active picker turns off as another photo opens.
- **A tool picker measures twice:** at its operation's place, for the tool; and on the frame, for the Colour picker, which shows the colour under the active picker whichever picker it is, as darktable's utility shows its primary sample.

## Regions

**The photo's coordinates.** Every command and event names a point or an area in the photo's coordinates: 0 to 1 across the photo as export renders it, after crop and the geometry, (0, 0) its top left, x to the right and y down. They do not depend on the zoom, the window or the photo's size, so a script and the app name the same place.

**Kept in the original image's coordinates,** as darktable keeps them (`color_picker.c:307-330`): the session maps a point, or an area's four corners, back through every geometry's `backward` (decision 16) to demosaic's output at full resolution, 0 to 1, clamped. So an area stays on the same part of the scene when the photo is rotated, cropped or flipped later, as in darktable.

**Mapped forward where it is measured,** as `dt_color_picker_box` maps it (`color_picker.c:394-454`): the four corners through each geometry up to the operation, darktable's `DT_DEV_TRANSFORM_DIR_BACK_INCL` for the input and `_EXCL` for the output (the same for every operation with a picker, none of which has a geometry of its own), sorted, the two lowest and the two highest averaged on each axis, scaled to the buffer's pixels at its pipe's scale, at least one pixel wide and high, clamped to the buffer. An area wholly outside the buffer measures nothing. A point is the one pixel it falls in.

**Shown as drawn,** as `dt_color_picker_transform_box` draws it (`color_picker.c:350-391`): the event gives each region forward-mapped to the photo's coordinates, an area as the average of its sorted corners, so a client draws a rectangle on the frame with no geometry of its own (the `desktop-architecture` skill: points go through the engine).

```rust
/// In the photo's coordinates, 0 to 1.
pub enum Region {
    Point([f32; 2]),
    /// Left, top, right, bottom.
    Area([f32; 4]),
}
```

**Defaults,** darktable's (`lib.c:1602-1612`): a point at the photo's centre, an area from 2 % to 98 % of it on each axis, mapped back as any region is.

**Built** (79.1, `apertrust_engine::picker`): `Geometries` maps a point back and forward through every operation that moves pixels, each at its input's extent at full resolution, which today is orientation's `flip` alone; a geometry that gives a point no place leaves it where it is. `Place` keeps a point, or an area's four corners, in the original image's coordinates, and gives the region as drawn; `pixels` makes darktable's box in a buffer from it, its sides truncated toward zero as C converts a float, so a point on the photo's right or bottom edge falls in no pixel and measures nothing, as in darktable. Each picker's point and area are kept for the session's life, across photos, as darktable keeps its pickers' places. 79.3 maps a tool picker's region forward through the geometries before its operation only, into the operation's stage, then `boxed` makes the box in the operation's buffer: at the pipe's scale and held to the buffer's region after the downscale, and in the raw stage on the sensor, from the declared area's corner to the mosaic's own.

## Measuring on the GPU

### Where

**A tool's picker measures at its operation's place in the live viewport pipe:** the operation's input, and with `output` its output, in the buffers the viewport's plan already holds, never a host copy of the image. Only a small result comes back: three statistics of four channels, 48 bytes a region.

- **The operation's input or output buffer** is what the reduction reads, right after the operation's last pass in the render's encoder, while both are intact: the planner writes an operation's output into a buffer that is not its input (engine.md, Planning), and nothing writes either before the next operation runs.
- **Its colour space** is the stage's: the mosaic in the raw stage, camera RGB, the working space, as each operation's slot names it (decision 21). A picker's `space` converts each pixel before its statistics, as `picker_cst` does ([The kernel](#the-kernel)).
- **The frame samples,** the Colour picker's, read the display buffer the frame was copied into, after colorout and before any rounding to 8 bits, as the scopes count it (decision 19): darktable's gamma input in its preview pipe (`pixelpipe_hb.c:3017-3027`). In a zoomed view, past Fit, they read the fitted pipeline's frame, `Frame::overview`, which the scopes count then (decision 99), never the visible region alone, so their values are the same at every zoom. Denoised where the sample is: the active picker's with its picker's denoise, a stored sample with the denoise it was copied with (`libs/colorpicker.c:613`, `pixelpipe_hb.c:1154`). Today the frame is sRGB, which is darktable's default display and histogram profile, so its values are both darktable's `display` and `scope` RGB.

### The scale

**The fitted viewport's,** darktable's preview pipe's place in Apertrust (decision 63: the export's processing at the view's scale, never darktable's preview shortcuts):

- **After the downscale** (every operation after demosaic: Exposure, Colour calibration, the tone mapper, Colour balance RGB, the curves): at the viewport's size, 2400×1600 for a full-screen photo, which is darktable's preview pipe's role.
- **In the raw stage** (White balance over `temperature`): at full resolution, since the raw stage runs there in every pipe (decision 12). White balance's mean is then the export's photosites' mean over the area, where darktable's is its preview's downscaled mosaic's; the two agree but for the area's edges, and a minimum or a maximum, which White balance does not read, would differ more.
- **A zoomed view** still measures the fitted whole image, built past Fit: while zoomed past Fit the fitted viewport pipeline keeps rendering beside the detail pipeline that draws the visible region (decision 99, [engine.md](engine.md#the-zoomed-view)), so it holds every buffer a picker reads at Fit, with the same exception, a tiled viewport's raw-stage tiles ([Memory, strips and failures](#memory-strips-and-failures)), and its frame, `Frame::overview`, is the one the frame samples read. An area outside the visible part measures, as darktable's preview pipe does, and a pick gives the same values at every zoom.
- **Exports, stills, tiles, golden cases and derivative passes** never measure: pickers are the live viewport's, as readouts are.

### The kernel

The engine's own kernel, `crates/engine/slang/picker.slang`, one for every operation, as the scopes' kernel is for every frame (in `pipeline::KERNELS` for the naga test). It is no operation's pass: no derivatives, no golden case, no fusion.

| Entry point | Reads | Writes |
| --- | --- | --- |
| `picker_rgba` | An RGBA `f32` buffer and up to 64 boxes in its pixels, with each box's space and denoise flag | Per box and per workgroup: the sum, minimum and maximum of each of four channels |
| `picker_mosaic` | The mosaic, its CFA pattern (Bayer `filters` or the X-Trans 6×6) and the boxes | The same per CFA colour, with each colour's count |
| `picker_combine` | The partials | Per box: the mean, minimum and maximum, in a fixed order |

- **Statistics** are darktable's `_update_stats_by_ch` (`color_picker.c:35-44`): the mean is the sum over the box's pixels divided by their number, or on a mosaic each CFA colour's over its own count (`:299-304`); the minimum and the maximum are exact.
- **The sums** are pairwise, a tree in workgroup memory; then one workgroup a box adds its workgroups' partials, each invocation those 256 apart in their index order, then the 256 as a tree again, so the result never depends on scheduling. On the CPU the same tree gives the same bits; across GPUs `f32` addition is correctly rounded, and a backend that contracts or reorders adds may differ by a few units in the last place, which no pick's write can feel. The kernel writes each box's sum, and the engine divides it by the box's pixels on the CPU, as darktable's `acc / (float)size`, since a GPU's division need only be within 2.5 ulp.
- **Denoise** (`DT_COLOR_PICKER_DENOISE`) is darktable's `blur_2D_Bspline` at one scale (`color_picker.c:480-502`, `src/common/bspline.h`): the 5×5 B-spline, weights 1, 4, 6, 4, 1 over 16 on each axis, its edges as darktable's, negatives kept. darktable blurs the whole image first; the kernel blurs only the box's pixels, reading two pixels round it, which gives the same values.
- **Spaces,** darktable's `dt_color_picker_helper` (`color_picker.c:505-543`): `Space::Operation`, the stage's own values; `Space::Lch` for a Lab stage (`dt_Lab_2_LCH`); `Space::Hsl` for an RGB one (`dt_RGB_2_HSL`); `Space::Jzczhz` for an RGB one through the working profile's matrix to XYZ D50, D65, JzAzBz and JzCzhz (`:59-82`). In each converted space the fourth channel is the third moved half a turn, `[2] < 0.5 ? [2] + 0.5 : [2] − 0.5`, as `_update_stats_4ch` makes it (`:53`): the hue in LCh and JzCzhz, but L in HSL, whose third channel is lightness. In `Space::Operation` the fourth channel is the stage's own fourth, and on a mosaic the fourth CFA colour's. A mosaic is read per CFA colour only (`:547-556`). The spaces other than `Operation` come with their first picker ([The plan](#the-plan)).
- **Rules:** `fp32`; no write at a run-time index, the four channels through `set_at` and `add_at` (decision 10); no NaN checks, the operations before it sanitising their outputs; the `slang-kernels` skill's "A count" adapted to sums, minima and maxima.
- **Built** (79.1): `picker_rgba` and `picker_combine`, in the buffer's own values with no denoise, the frame's case. Each invocation adds 4 × 4 pixels 16 apart, so a workgroup covers 64 × 64 of one box; each box's workgroups follow the last box's in one dispatch, as many across as the device allows, then down. The uniforms hold the pass header, then the boxes, each its bounds and its first workgroup, workgroups across, count and flags. The sums, and so the means, the minima and the maxima, are the CPU's restatement's to the bit on the M4.
- **Built** (79.3): `picker_mosaic`, each photosite added to its CFA colour's channel by `add_at` and `set_at`, its colour from the pattern at the mosaic's place on the sensor, Bayer or X-Trans, from the uniforms; the engine counts each colour's photosites in the box on the CPU from the pattern, darktable's `weights`, and a colour with none has a mean of 0 and the minimum and maximum the kernel starts from, as darktable's. Denoise, a box's flag, blurs each pixel the box reads from the 25 around it, clamped to the buffer, before its statistics, within 4 units in the last place of the restatement on the M4, where a GPU may fuse the multiply-adds. One reduction reads up to two buffers, an operation's input and output, each its own pair of passes, its results after the last's. The sums of a mosaic's boxes are the restatement's to the bit; dispatched across and down, under a test's cap on workgroups across, they are the same bits as in one row.

**Several boxes in one dispatch:** the tool picker's one box at its operation, and on the frame the active picker's and every unlocked stored sample's, so the frame is read once however many samples there are. Colour calibration's colour checker will read its patches the same way, up to its chart's patches in one dispatch.

### When it runs

A reading has a key, as a readout's does (decision 19): what it reads, the box in that buffer's pixels, its space and denoise. A reading runs only while its key differs from the last one measured.

- **A render** that rewrites the picked operation's input or output (a change at or before the operation) dispatches the tool picker's reduction in the same encoder after the operation's last pass. A render that restarts after the operation, as a drag of a later tool does, leaves its key unchanged and measures nothing at the operation.
- **A region change with no edit** needs no frame, since the frame is unchanged: the session runs a sample-only render, from the newest cache point at or before the operation through the operation, then the reduction, and publishes no frame, as a readout asked without a current reading does (decision 19).
  - **Turning a tool's picker on makes its operation the edited instance** (engine.md, Caching), as darktable focuses the module (`color_picker_proxy.c:228`). `On` runs a capture: its sample-only render copies the operation's input into the edit cache, as the first render after a change does. After the downscale an input reading of a moved area then reads the edit cache directly, with no pass of the pipeline, and an output reading runs the operation's own passes from it.
  - **The override lasts** until another operation's key changes, which makes that operation the edited instance as any change does. After a rebuild, which captures nothing (engine.md, A rebuild keeps what still holds), the first moved area renders from the newest cache point at or before the operation, and captures again.
  - **Sample-only renders submit as frames do,** through `Pipeline::writes` and the submit gate, one turn each, behind the live frame and before background stills.
  - **In the raw stage** there is no edit cache: White balance's moved area reruns rawprepare from the uploaded mosaic, then measures temperature's input.
  - **Built** (79.3, `Pipeline::render_probing` and `Pipeline::sample`): the key hashes the operation's input key, and its output key with `output`, each box, denoise and `output`. A render that runs the operation records the reduction right after its last pass, its readback's copy in the same submission, so the frame and the reading take one turn. A sample-only render reads the edit cache with no pass where it holds the operation's input and only the input is read; otherwise it runs from the newest cache point before the operation, or from the edit cache where it holds the input of the operation or of one before it, up to the operation, and through it where its output is read; it captures the operation's input where the edit cache takes it, and changes no cache point after the last operation it ran. Before it, `Pipeline::sample_writes` cancels a still that reads what it writes, as `Pipeline::writes` does before a frame. It is not waited for: its reading arrives as a frame's does. Where the last render did not render the plan as it is, it runs nothing, and the next frame measures.
- **The frame samples** are measured after each frame while the active picker or an unlocked stored sample is there, in a submission of their own after the frame's copy and the scopes' count (decision 19), and on a region or sample change on the frame shown, with no render. Built for the active picker: the frame shown is measured again after a picker command that moved its region in a batch that publishes no frame, none rendered or the render failed.
- **A locked stored sample** is not measured again; the session keeps its last reading.

**Readback** is the scopes' and the readouts': the results copied into whichever of the pickers' two staging buffers is free, mapped and polled between commands, never waited for, the latest replacing the last while both are busy, so the last reading always arrives (`crates/engine/src/readback.rs`, the helper the scopes, readouts and pickers share; built, the scopes' `Meter` and the pickers' `Reducer` on it). A frame never waits for a reading. The session keeps two reducers, the frame's and the operation's, each with its own buffers and staging pair, so a reading at the operation waiting for a staging buffer is never replaced by the frame's; the operation's records its copy in the render's encoder (`Readback::record`) and maps it once the encoder is submitted (`Readback::sent`).

### Memory, strips and failures

- **Buffers:** the boxes' uniforms, the partials and the results, the session's own, counted in the plan's memory as a readout's counts buffer is. The partials take 48 bytes a workgroup, at most the frame's 64 boxes in one dispatch, about 2.9 MB at 2400×1600, which also holds one box over a 61 MP mosaic at 64 × 64 photosites a workgroup; more boxes take more dispatches. Built (79.3): each reducer grows its partials to what a reduction needs, 45 KB for the whole 2400×1600 frame and 715 KB for one box over the 7R V's mosaic, the input's and output's reductions sharing them; the viewport is planned within `engine.memory_budget` less `picker::reserved`, the two reducers' most for the viewport's size and the photo's mosaic: the frame's partials for 64 boxes over the whole frame and the operation's for one box over the larger of the frame and the mosaic, their results and staging pairs, and their uniform blocks, one a dispatch, at worst a window a row of workgroups for the operation's input and output, about 4.1 MB at 2400×1600 for the 7R V. A compile as a photo opens plans within the same. The reducers' partials and uniform blocks only grow, never shrinking for a smaller photo, so after a larger one the reserve counts up to about 1 MB less than they hold.
- **Strips:** a buffer larger than one binding, as the 61 MP mosaic on lavapipe, is bound in row windows, each window's workgroups writing their partials at their index in the whole box, so the result is the whole box's (decision 14). Every buffer after the downscale is at the viewport's size, so a picker there never reads a tile. Built (79.3): a tiled viewport (decision 14) runs the raw stage's operations from its split on once a tile, in the tiles' own buffers, as on lavapipe with a 61 MP raw or within a small memory budget, so a picker whose operation lies in the tiles, as temperature does where no whole-image pass keeps the raw stage whole, gives no reading at its operation: its reading is sent with no tool part, nothing is applied, and the session logs it under `pipe`. Windows: box by box, strips of whole workgroups' rows, each a dispatch over the window of the rows it reads, a denoised box's two rows either side included, its offset aligned to the device's; a window that cannot hold one workgroup's rows refuses the reduction. Opened to bind 256 KiB, a mosaic and an RGBA buffer, denoised and not, give the bits they give bound whole.
- **Failures:** a reduction the GPU refuses, inside the frame's submission, makes the session render that frame again without pickers, then turns pickers off for the session, reported once as `SessionError::Pickers`; a mapping that fails does the same. A picker never costs the frame, as the scopes and readouts never do. Built for the frame samples, in a submission of their own: a reducer that cannot be made, a reduction the GPU refuses or a mapping that fails turns the pickers off for the session, reported once as `SessionError::Pickers`, followed by a `Picker` with no reading where one was on; the frame is published all the same, and `On` is refused from then on with the same error. Built (79.3) at the operation: a render whose submission the GPU refuses is rendered again from a new pipeline without the reduction only where one was in it; where that one renders, the reduction was to blame and the pickers turn off, and where it fails too, or no reduction was in the submission, the error is the frame's and the pickers stay as they were; a reduction whose windows cannot bind, a sample-only render the GPU refuses, or a mapping that fails turns them off the same way, the frame unaffected; a pick that is refused turns the picker off ([What a pick writes](#what-a-pick-writes)).

### Cost

The frame samples measured in 79.1, the moved area's budgets in 79.3, both on the M4:

- **Frame samples,** measured on the M4 at 2400×1600, both passes' GPU timestamps, median of 50 after 10 to warm up, in two runs (`cargo test --release -p apertrust-engine --test picker -- --ignored --nocapture`): a box over the whole frame, its 61 MB read once, 0.98 and 0.99 ms of GPU time (min 0.96); the default area 0.95 and 1.12 ms (min 0.91); a 5 % area 0.09 ms; a point 0.08 ms, the two passes' fixed cost. A reading is picked up 1.8 to 1.9 ms after its submission for the whole frame, 0.4 ms for a point, when nothing else runs. A whole frame costs half again the scopes' totals pass, 0.64 ms, for the tree's barriers and three statistics where the scopes count.
- **The budgets:** a moved area's reading within 16 ms of its command after the downscale, and within 50 ms in the raw stage. Measured on the M4 with the 7R V's 61 MP raw (`7RM5-LosslessCompressedLarge.ARW`) at 2400×1600, each of 50 moves of an area of 90 % to 96 % of the photo on each side after 10 to warm up, from the command to its reading at the operation picked up as the session's thread picks it up, in two runs (`cargo test --release -p apertrust-engine --lib -- moved_area --ignored --nocapture`):

| Moved area | Median | Min | Max |
| --- | --- | --- | --- |
| Exposure's input, after the downscale: the edit cache read with no pass | 1.38 and 1.68 ms | 0.13 ms | 2.10 ms |
| temperature's input, in the raw stage: rawprepare from the uploaded mosaic, then about 52 MP of photosites read | 8.98 and 10.04 ms | 8.55 ms | 10.51 ms |

- **Within both,** so the raw stage keeps no cache point for a picker's operation: a moved area reruns rawprepare each time.
- **White balance's pick,** once applied, changes the coefficients, and that change reruns the raw stage and demosaic at full resolution, about 240 ms at 61 MP on the M4, as any White balance change does (engine.md, Where the viewport's scale applies).

## Through the session

### Commands

`Command::Picker(PickerCommand)` and `Command::Samples(SamplesCommand)`. Built: `Command::Picker` with `On`, `Point`, `Area`, `Switch`, `Reset` and `Off` for `PickerRef::ColourPicker` (79.1), and for `PickerRef::Tool` with `On`'s `settings` and `channel`, `Settings` and `Channel` (79.3), as below; `Samples` is planned (79.14). Neither is a command on the edit (`on_edit`): they are view state, as `Scopes` and `Readouts` are, so they end no gesture. `On` is refused before the photo is live and, for a tool's picker, on a photo open for viewing only and while the History tab's eye hides a tool (below); the Colour picker's picker and `Samples` work on a photo open for viewing only. A pick's own write is a change on the edit ([What a pick writes](#what-a-pick-writes)).

```rust
pub enum PickerRef {
    /// A module's picker, by its module's id and the picker's id in its definition.
    Tool { tool: &'static str, picker: &'static str },
    /// The Colour picker module's, darktable's primary picker with no module.
    ColourPicker,
}

/// The keys held, which some modules read as darktable reads `dt_key_modifier_state`.
pub struct PickKeys {
    /// Ctrl, Cmd on the Mac.
    pub command: bool,
    pub shift: bool,
}

pub enum PickerCommand {
    On { picker: PickerRef, area: bool, keys: PickKeys, settings: Vec<(&'static str, ControlValue)>, channel: Option<&'static str> },
    Point { at: [f32; 2], done: bool, keys: PickKeys },
    Area { from: [f32; 2], to: [f32; 2], done: bool, keys: PickKeys },
    Switch,
    Reset,
    Settings { tool: &'static str, settings: Vec<(&'static str, ControlValue)> },
    Channel { channel: Option<&'static str> },
    Off,
    /// A tool's picker on for one reading of `region`, applied, then off.
    Once { picker: PickerRef, region: Region, keys: PickKeys, settings: Vec<(&'static str, ControlValue)>, channel: Option<&'static str> },
}
```

| Command | What it does |
| --- | --- |
| `On { picker, area, keys, settings, channel }` | Turns the picker on, the one on before it off. A picker with both kinds takes an area with `area`, else a point; one kind, that kind. It starts at the picker's last point or area this session, else the default. Its tool is turned on, as darktable's is, and its operation becomes the edited instance. `settings` are the tool's picker settings ([A tool's settings](#a-tools-settings)); `channel` is the channel tab the tool shows, RGB levels' and RGB curve's. Its first reading is applied, one completed change |
| `Point { at, done, keys }` | The active picker's point moved, a click or a drag's step; `done` on the release |
| `Area { from, to, done, keys }` | The active picker's area from one corner to the other, a drag's step or its release; any order of corners |
| `Switch` | For a picker that takes both kinds: point to area or back, each at its last place, darktable's Ctrl-click on the photo (`darkroom.c:4017-4034`), which the app is to stop sending from the photo (decision 105, planned); a completed change. Refused for a picker that takes one kind (planned, below) |
| `Reset` | The picker's default for its kind: the default area, darktable's right-click on the photo with an area (`darkroom.c:4125-4132`), or the default point, which Delete alone reaches, darktable's right-click doing nothing with a point; a completed change |
| `Settings { tool, settings }` | The tool's picker settings changed: kept, and the tool's latest reading measured applied again where its pick reads them, as Exposure's target lightness in correction mode applies again (`exposure.c:1154-1176`). The session keeps each tool's latest reading after its picker turns off, for the photo, as darktable keeps `picked_color`; with none, nothing is applied |
| `Channel { channel }` | The channel tab the active picker's tool shows changed, as the client sends `settings`; kept for its next pick, applying nothing, as darktable's tab switch applies nothing (`rgblevels.c:756-766`) |
| `Off` | The active picker off; its point and area kept for its next `On` |
| `Once { picker, region, keys, settings, channel }` | A one-shot pick, a tool's picker on for one reading of `region`, a click's point read as darktable's click reads it, its pick applied, then off, one completed change and one step where it changes the edit, none where it changes nothing; its picker's own point and area untouched ([A one-shot pick](#a-one-shot-pick)) |

- **Built,** in the details: `Point` and `Area` set the active picker's point or area and make it measure that kind, its other place kept, and `Switch` changes the kind, whatever kinds the picker takes, as darktable's Ctrl-click on the photo switches the primary sample whatever its picker (`darkroom.c:4017-4034`); the kinds limit only `On`. Planned (decision 105): a picker that takes one kind keeps it, its `Switch`, a `Point` for an area picker and an `Area` for a point picker refused (below); `Once` unchanged. `On` for the picker already on starts it again, its first reading applied. `Off` with no picker on changes nothing and sends nothing; `Channel` with none on changes nothing; `Settings` with no reading of the tool kept, or none whose picker reads a setting named, applies nothing.
- **Refused,** each as `SessionError::Refused` with the command's number: `On` before the photo is live (`NotLive`) or, for a tool's picker, on a photo open for viewing only (`ViewOnly`), as commands on the edit are (decision 77), and while the History tab's eye hides a tool (`Hidden`, decision 78: it would read the view and write the edit; [history.md](history.md#commands)); a picker no tool declares, or whose operation the plan lacks after its tool is turned on (`UnknownPicker`); a point or area command, `Switch` or `Reset` with no picker on (`NoPicker`); `On` or `Settings` naming settings the tool's pickers do not read (`UnknownSetting`); `On` for a picker its tool says is not ready for the settings sent (`NotReady`, `ProcessingTool::picker_ready`, decision 88), as Exposure's toward a reference with none set. Planned (decision 105): `Switch` for a picker that takes one kind, `Point` for an area picker and `Area` for a point picker (`OneKind { takes }`, naming the kind the picker takes: "the colour picker takes only an area"), its kind and place left as they were.
- **The region** is in the photo's coordinates, clamped to 0 to 1. A drag sends its steps with `done: false` and its release with `done: true`; the session makes the steps one gesture.
- **Keys** ride with every command that can apply a reading, so a tool reads them as darktable reads the modifiers when its reading arrives: Colour balance RGB's hue picker takes the colour itself with `command`, else its opposite (`colorbalancergb.c:1289-1290`); RGB curve's create-curve picker makes a positive curve with `command`, a negative one with `shift` (`rgbcurve.c:557-564`). On the photo a Ctrl-click is a plain click carrying `command`, for every picker (decision 105, planned; built, it sends `Switch`).

```rust
pub enum SamplesCommand {
    Add,
    Lock { id: u32, locked: bool },
    Load { id: u32 },
    Store { id: u32 },
    Remove { id: u32 },
    Clear,
}
```

| Command | What it does, after `libs/colorpicker.c` |
| --- | --- |
| `Add` | The active picker's point or area as a new stored sample, unlocked, last in the list, with the active picker's denoise (`_add_sample`, `:597-673`, `:613`); refused with no picker on, as darktable's add button is insensitive then (`:268-269`) |
| `Lock { id, locked }` | A sample locked keeps its reading and is no longer measured (`:527-531`) |
| `Load { id }` | darktable's first right-click on a sample's patch (`:539-582`): the sample's place into the Colour picker's own picker, turning it on whenever it is not the active one, a tool's picker on then being turned off; the sample marked loaded. On the photo, a right-click on a sample while samples are shown loads its place into the active picker instead, whichever it is, only where its kind is one that picker takes (`darkroom.c:4087-4123`) |
| `Store { id }` | darktable's second right-click while a sample is loaded (`:543-558`, `:573-582`): the Colour picker's place into the sample, then the Colour picker's picker turned off |
| `Remove { id }` | The sample gone (`:508-521`) |
| `Clear` | Every sample gone and the Colour picker's picker off, its reset (`gui_reset`, `:887-929`) |

**Ids and numbers.** `id` is a sample's stable handle in commands and events, counting from 1 in the session and never reused. The number shown beside a sample, and in its accessibility names, is its place in the list, from 1, as darktable lists them, so removing one renumbers those after it.

### Events

Two facts ([decision 59](decisions/0059-one-event-system.md)), each its whole topic:

```rust
/// Fact::Picker. `None` while no picker is on.
pub struct PickerReading {
    /// As `Frame::through` counts them, every command the session had applied when it measured: the frame's, and any after it that rendered nothing.
    pub through: u64,
    pub picker: PickerRef,
    /// As drawn, in the photo's coordinates.
    pub region: Region,
    /// A module picker's statistics at its operation; `None` for the Colour picker's.
    pub tool: Option<ToolReading>,
    /// The frame's colour under the region; `None` until measured, or where the region misses the frame.
    pub frame: Option<FrameReading>,
}

pub struct ToolReading {
    /// `None` where the region misses the operation's image, as darktable's empty box.
    pub input: Option<Stats>,
    /// With the picker's `output` only.
    pub output: Option<Stats>,
    /// What the module's pick made of the reading for its panel; `None` where it made nothing.
    pub picked: Option<Shown>,
}

/// darktable's `lib_colorpicker_stats`: per channel, in the picker's space, the fourth
/// channel a hue moved half a turn where the space has a hue, else the stage's alpha,
/// or on a mosaic the fourth CFA colour.
pub struct Stats {
    pub mean: [f32; 4],
    pub min: [f32; 4],
    pub max: [f32; 4],
}

/// darktable's `display`, `scope` and `lab` of a sample: mean, min and max.
pub struct FrameReading {
    /// The frame's RGB, as the window shows it and the scopes count it.
    pub rgb: [[f32; 3]; 3],
    /// Each of the three converted to Lab D50, as darktable converts mean, min and max.
    pub lab: [[f32; 3]; 3],
}

/// Fact::Samples: every stored sample, in the order added; its number shown is its place here.
pub struct StoredSample {
    pub id: u32,
    pub region: Region,
    pub locked: bool,
    pub frame: Option<FrameReading>,
}
```

- **Built:** `Event::Picker` with `through`, `picker`, `region` and `frame` (79.1), and `tool` (79.3); `Event::Samples` comes with stored samples (79.14). `through` is every command the session had applied when it measured: the frame's, and any after it that rendered nothing, so a reading of the frame shown after a move includes the move. A command that changes nothing brings no reading with a newer `through`, so a client must not wait for one. A reading goes out when its region, its tool part or its frame part differs from the last sent; the tool part and the frame part arrive apart, each a reading carrying the latest of the other, so after a move one may carry the other's part of the last region until its own arrives. A tool part's `picked` is what the last reading applied made, kept by the readings an edit caused. A `Picker` with no reading goes out in the batch that turns the picker off, its `through` every command applied then, so a client knows a picker it just turned on was turned off again before its first reading, as another photo opens before that photo's events, and the readings on their way are dropped.
- **`Event::Picker { through, reading: Option<PickerReading> }`** is sent when a picker turns on, once its first reading arrives; after each reading that changed its tool part or its frame part; and with no reading as it turns off. During a drag of another tool the frame part changes every frame, so a reading follows each frame while a picker is on, about 200 bytes.
- **`Event::Samples(Vec<StoredSample>)`** is sent after each command that changes the list and after each frame that changed an unlocked sample's reading.
- **Order:** a reading after the `Frame` it measured, as the scopes' readings come, and before the `Values` of the change a pick applies. Readings of a photo that closed never arrive.
- **The types** live in `apertrust_engine::picker` and `apertrust_tools::pick`, re-exported by the session: plain data that moves into `apertrust-protocol` unchanged. `Region`, `PickKeys`, `Stats` and `Shown` are `apertrust-tools`'s, since `ProcessingTool::pick` takes and gives them.

### Turning off

The session turns the active picker off, sending a `Picker` with no reading, when:

- `Off`, or `On` for another picker;
- another photo opens, or the photo closes (`darkroom.c:1100-1102`);
- its tool changes by any command other than its own picks: a control set, reset or kept, a preset, `ResetTool`, `Reset`, Undo, Redo or a `GoTo` that changes it, as `dt_iop_gui_changed` and `gui_update` reset it; unless the picker is `keep`, darktable's "keep-active";
- its tool turns off, since darktable picks only while the module is enabled (`pixelpipe_hb.c:1194-1196`).

Built (79.3, and Undo, Redo, `GoTo` and the history's rewrites with decision 104's 104.5): each turns it off where it changes the picker's tool. A command on the edit that names the picker's tool, `SetControl`, `ResetControl`, `ResetTool`, `SetTool`, `Keep`, `ApplyPreset` or `CancelGesture`, or `Reset`, turns it off once applied, never one that is refused; a switched tool turns off only through such a command, so that covers it. `Set`, of an operation's parameter, names no tool.

**Focus is the client's,** as the desktop-architecture skill has it: Apertrust has no focused tool, so in place of darktable's module losing focus (`imageop.c:2414-2416`) the app sends `Off` while a tool's picker is on when another tool's control changes, when another tool's Show on photo turns on (planned, with Rotate and perspective's port; one tool's on at a time), or when the picker's own tool's section is collapsed in the Tools panel, whichever door turned it on (decision 104); and while the Colour picker's picker is on, when any tool's control changes or Show on photo turns on, unless Restrict scope is on, as darktable's any focus change (`imageop.c:2393-2395`), or when its own section is collapsed, Restrict scope or not: one rule for every picker, Restrict scope the Colour picker's opt-out from other tools' changes. Planned, the Colour picker's with its section (79.16). A script that never focuses keeps the picker through focus changes; the session's own turn-offs above still end it.

### What a pick writes

**The session applies a tool's reading itself,** on the engine thread, as the reading arrives: it calls the tool's `pick`, a pure function as `bind` is, and applies what it returns to the edit. Every client gets the same write; no client runs tool code.

- **Which readings apply,** darktable's `_record_point_area` rule (`color_picker_proxy.c:76-104`, `:305-327`): the first reading after `On`, the reading after each `Point`, `Area`, `Switch`, `Reset` or `Load`, after `Settings` its last reading again, and a `Once`'s one reading ([A one-shot pick](#a-one-shot-pick)). A reading measured because an edit changed the operation's input is sent, never applied, so a pick never feeds back on its own write.
- **A pick is a change of source Auto** ([history.md](history.md#merging-consecutive-changes)), its target the picker. A drag's readings are one gesture: each `done: false` region's pick is a gesture's change and the `done: true` region's completes it, one step, merged with a quick repeat on the same picker as a control's change is (decision 78). `On`, `Switch`, `Reset`, `Load` and `Settings` are completed changes, and so is `Once`, its turning on of the tool included, with no target, so it never merges. Where a release's region equals the last measured, no new reading is due, and the session completes the gesture with the values already written.
- **It counts as the `Picker` command that asked for it** for `through` and merging: the `Values` after it includes that command, and its step joins the gesture that command continues ([history.md](history.md#merging-consecutive-changes)).
- **A switched tool turns on** with the pick, as with any write of its controls (decision 32), and with `On` itself where the tool is off, as darktable turns the module on with its picker: that is the first step of a picker that only shows.
- **A value a pick gives that is not finite,** as `log2` of zero or less where darktable's rule has no floor, is not written; the pick's other writes and what it shows stand, so the patch and the readout still follow the area, as darktable shows them before it sets anything (`exposure.c:885-897`). A pick left with nothing to write makes no step.
- **A gesture under way is closed as a step** when the picker turns off or fails mid-drag, as any other command on the edit ends a gesture as completed (decision 42).
- **A picked value is stored as darktable stores it:** each tool's own rules apply, as in its `color_picker_apply` ([The first pass's pickers](#the-first-passs-pickers)), with no limit to a slider's soft range. The one bound is the parameter's hard limits, darktable's `$MIN` and `$MAX`, the control's range that the session already holds every value to (decisions 21 and 30): a value darktable's rule leaves past them is held at the nearer limit, a hue wrapped onto its turn instead, the same angle. A value beyond the soft range shows as any value set from outside does: the track widens to it, its handle at the end, and the value field shows the true value, as darktable's `dt_bauhaus_slider_set` does ([gui.md](gui.md#how-the-rows-are-laid-out)).
- **Refused writes** write nothing and turn the picker off, reported as `SessionError::Refused` naming the command that asked for the reading. A tool's picker cannot turn on while the photo is open for viewing only, nor while the History tab's eye hides a tool, and one on measures nothing and applies no reading while a tool is hidden, its reading due waiting until all are shown, since it would read the view and write the edit; the Colour picker's can, and its readings are sent there.
- **A reading measured before a change** that the picker's tool has since received is dropped, since that change turned the picker off.
- **Every completing command is answered** (79.6): a tool picker's `On`, `Switch`, `Reset` or a move's release gets a `Saved` through it once its drag is complete, released with no reading of it to come, and `Settings` at once where no change of the edit's own is unsaved, whether it applies a pick or not: the pick's write, or, where it wrote nothing, the edit reported held as after a cancel, written again where its last write failed, or, while a change of the edit's own is unsaved, as a slider's gesture under way, that change's save; a picker turned off or failing answers one still owed the same way. A write handed over while a command's pick is still to come is through the command before it, never answering it early. So the app waits for each as for any completed change, Saving… showing and Quit waiting to report a failed write, and none is left waiting. The Colour picker's commands write nothing and are never waited for.
- **Built** (79.3, and since the history is saved, 9.5): what a picker wrote is made one step of source Auto where the session saves it, as each drag's picks are handed to the writer below and as `On`, `Switch`, `Reset` and `Settings` save theirs, unmerged with a quick repeat until a pick's own gesture and target come ([history.md](history.md#what-makes-a-step)). A pick is written as a change with no follow writes (`Edit::pick`), each control kept; a drag's `done: false` picks leave the edit unsaved and the release's saves it, as a slider's gesture is saved; `On`, `Switch`, `Reset` and `Settings` save theirs. A pick also ends a slider's gesture under way. Each drag is saved once, on its release, never merged with the next: `On` and a move after a release begin a drag, each reading to apply carries its drag's number, and a drag's picks are handed to the writer once it is released and no reading of it is still to come, so a release with nothing on its way saves at once, and one whose reading misses the image or whose pick is dropped saves what the drag wrote; a later drag's pick saves an earlier drag's first, as `On` and `Settings` save a drag under way before their own change. A release that the next drag's first step replaces in the same batch is not measured: its drag is saved with the writes its steps made. Readings are handed over in the order of the commands they measured, each waiting for any measured before it still on its way, so a release that needs no GPU never overtakes its drag's step, and the last reading sent carries the latest region. A reading to apply that waits for a staging buffer and is replaced by a newer one passes its application on where the newer has none; a GPU error that drops the readings on their way asks again for the newest application among them, with the next measurement, which stands for all of them. A pick that panics, writes a control outside its picker's `writes`, or gives a value its control cannot take or an option the photo does not list is the tool's failure, `EditError::PickFailed`, refused as above; from `Settings`, the command is refused and the tool's picker turned off where it is on. A hue, a control in degrees over a whole turn, wraps; every other number is held to the range the tool's values give its control now. The session keeps two readings of each tool for the photo, cleared as another opens or it closes: its latest measured while its picker was on, applied or not, darktable's `picked_color`, which every run of the pipe overwrites (`pixelpipe_hb.c:1574-1580`), and which `Settings` applies again as Exposure's settings callback applies it (`exposure.c:1171-1175`), the reading then sent with what the pick showed; and its last applied, passed to `pick` as `last`, as RGB levels' `last_picked_color` (`rgblevels.c:788-796`), recorded only once a pick is written, so a write the edit refuses leaves it as it was. The tools' settings, each tool's as the client last sent them, last for the session.

### A tool's settings

Some pickers read darktable's own settings, kept in its darktablerc and never in an edit, or Apertrust's: Exposure's Picker target, Lightness and the reference's L, Colour calibration's area mode, channel mixing and target L, h and c (tools rule 12). They become settings in `settings.toml` under `[tools.<module>]`, declared in the settings schema with `owner = "tool:<id>"` (decision 43), and their rows name those settings in place of their `Row::Setting` kinds.

- The engine reads no settings file, so the client sends their values with `On` and `Settings`, as a command that uses a setting carries its value (decision 43).
- **A pick that measures** writes no control: it returns the settings it measured in `Shown::settings`, as darktable's measure modes write their target sliders and darktablerc (`exposure.c:900-934`), and the app keeps them in its settings. Exposure's measure mode is gone (decision 88); its `reference` picker measures this way, and so does Colour calibration's measure mode (79.9), handing back its target's L, C and h. A reference's L comes with the open photo's path, which the app keeps beside it.
- **`Settings` hands none back:** its pick applies the latest reading again by the settings the client has just sent, `Shown::settings` empty, as darktable's settings callback runs Exposure's pick again (`exposure.c:1170-1175`): a change of Picker target or Lightness picks the last area again toward the new target.
- **Built** (79.7, and decision 88's third step), Exposure's: `tools.exposure.target` (Picker target: `middle_grey`, the default, `lightness` or `reference`, `apertrust_tools::exposure::PickerTarget`) and `tools.exposure.lightness` (0 to 100 %, one decimal, 50 % by default, darktable's `darkroom/modules/exposure/lightness`), generated by the `apertrust-tools` crate's `build.rs` into `apertrust_tools::settings::registry`. Each `Row::Setting` finds its setting by its id, `tools.<module>.<setting>` (`settings::setting`), and a test holds the two to agree: kind, range, step, decimals, options and default. `tools.exposure.area_mode` went with measure mode; an old key in a settings file stays as an unknown key, read as nothing and left as it is. The app's store reads them with its own (`settings::ToolSettings`); one `--conf` gives keeps its value for the run, its row disabled and a measured value refused; their other rows are live, with or without a photo, a change kept at once and sent as `Settings` with the tool's settings as a gesture ends; a press on the picker sends them with `On`; and a measured setting is kept as the user's own, taken once from the reading whose pick measured it (`pickers::Picked`).
- **Built** (79.9), Colour calibration's: `tools.channelmixerrgb.area_mode` (Area mode: `correction`, the default, or `measure`, `apertrust_tools::pick::AreaMode`, darktable's `dt_spot_mode_t`), `channel_mixing` (on by default, darktable's `use_mixing`), `target_lightness` (0 to 100, 50), `target_hue` (0 to 360°, 0) and `target_chroma` (0 to 128, 0), each with one decimal, darktable's `darkroom/modules/channelmixerrgb/lightness`, `hue` and `chroma`, kept and sent as Exposure's are. darktable puts the area mode back to correction at every `gui_update` (`channelmixerrgb.c:3779`); Apertrust keeps it, as it keeps Exposure's settings. The crate's `build.rs` generates both tools' settings into one registry (`codegen::build_all`).
- **A setting's rows:** rows may follow a choice setting, shown while it holds one of the options named (`Row::WhenSetting { setting, is, rows }`), as `Row::When` follows a control: Exposure's Lightness under Picker target while Lightness is chosen. `ProcessingDefinition::check` holds the setting to a choice `Row::Setting` of the tool, shown wherever the rows are, and the rows to hold no control, since what shows on the edit never follows the app's settings. An option still to come is listed disabled, "Coming in a later version", marked in its kind's `unported`.
- **A reference** (decision 88, `Row::Reference { picker, lightness, photo }`): two settings of the tool, an optional number holding an area's L and an optional text holding the path of the photo it was measured on, set together as the row's picker hands the L back and the app adds the open photo's path, cleared together from the row's ⋯, and kept across photos, folders and restarts: both or neither, so neither changes where `--conf` gives one, the row, its picker and ⋯ then drawn disabled, "Set by --conf for this run", and an L measured with no photo open comes with no path. One with no path, as `--conf` gives, shows its L alone, its picker's words the option's `-no-photo` ones (`messages::picker_option_no_photo`); one measured on the open photo says "this photo" in them (`picker_option_this_photo`). Built, Exposure's: `tools.exposure.reference_lightness` (0 to 100 %, one decimal) and `tools.exposure.reference_photo`, none until set. The app sends the L with the tool's settings and never the path; it asks whether the photo is still there off the interface's thread, as each photo opens and as a reference is set (`pickers::ReferencePhotos`), and dims the name once it is gone.

### A one-shot pick

Built (79.17), the session's side and `Region::pressed`. One command for a client that names a region and wants one pick, a script, the MCP or an assistant: the tool's picker reads the region, its pick is applied, and the picker is off again, one completed change and one history step where the tool or its values change, none where nothing does. The reading it applies is the one pressing the picker's button, clicking or dragging the photo there and pressing the button again applies for that region, with nothing left on; the button's way also applies a first reading at the picker's held area, which `Once` does not. The app sends it from nowhere: its picker cursors are doors to `On` ([cursors.md](cursors.md)).

```rust
PickerCommand::Once {
    /// A tool's picker: the Colour picker's writes nothing and is refused.
    picker: PickerRef,
    /// In the photo's coordinates, clamped to 0 to 1: a click's point or a drag's area.
    region: Region,
    keys: PickKeys,
    settings: Vec<(&'static str, ControlValue)>,
    channel: Option<&'static str>,
}
```

One command, not a sequence of the others: `On` applies a first reading at the picker's last place before any move, `Off` drops the reading on its way, so a client would have to wait for it between them, and the sequence would move the picker's kept place and draw its marks and move the focus for a moment.

- **What it reads:** an area as sent, whatever the picker's kinds, as `Area` reads it. A point as a plain click there reads in darktable: for a point picker, and one with both kinds, the point; for an area picker, the area a press elsewhere starts on the photo, 2 % of the photo's width each way of the point and as tall in the photo's pixels, clamped to the photo (`darkroom.c:4064-4073`, darktable's `dx` of 0.02 and `dy` scaled by the processed width over the height). One function in `apertrust-tools`' `pick`, `Region::pressed(at, aspect)`, makes that area for `Once` and, as built, for the app's press on the photo. Planned (decision 105), the app's press keeps that area's size at Fit on screen at every zoom ([On the photo](#on-the-photo)), so the two agree at Fit, and `Once` keeps darktable's 2 % of the photo at every zoom. The region is mapped back through the geometry and forward to the operation as every picker's is ([Regions](#regions)), and measured in the fitted viewport pipe at every zoom ([The scale](#the-scale)) by a sample-only render at its operation, as `On`'s first reading is. No frame part is measured.
- **What it writes:** what the tool's `pick` gives for that reading, as the picker's first reading after `On` writes, by the settings and channel sent with it ([What a pick writes](#what-a-pick-writes)). Its tool is turned on where it is off, or applied where the history's end lies among the default steps, as `On` turns it on. The turning on and the pick's writes are one completed change of source Auto, one step of its tool. The turning on is held until the pick is written or found to write nothing, so it is a step of its own only where a command comes first (Ended early, below): where the pick writes nothing, the step holds the turning on alone, and where it neither turns the tool on nor writes, there is no step.
- **The step has no target,** as `Reset`'s has none ([history.md](history.md#merging-consecutive-changes)): two one-shots never merge, since each is one deliberate press, and one Undo takes back each that makes a step.
- **Then it is off:** once its reading is applied, arrives with nothing to apply, or misses what it reads, as in a tiled viewport's raw stage ([Memory, strips and failures](#memory-strips-and-failures)). The picker's own point and area are left as they were, so its next `On` starts where the person left it.
- **Its events,** in the batch its reading arrives in: `Event::Picker` with its reading, its region as drawn and its tool part with what the pick showed (`ToolReading::picked`), so the tool's patch and Last pick follow it as they follow any pick, with no frame part; the change's `Values` and `History`, then its `Frame`; then `Event::Picker` with no reading. A client that keeps only a fact's latest value misses the reading, and with it the patch and Last pick, though not the change it wrote.
- **Answered** as `On` is: a `Saved` through it once its write is handed over, or the edit reported held where it wrote nothing ([What a pick writes](#what-a-pick-writes)).
- **Refused,** writing nothing, with `On`'s reasons: before the photo is live (`NotLive`), on a photo open for viewing only (`ViewOnly`), while the History tab's eye hides a tool (`Hidden`), for the Colour picker's picker, one no tool declares or one whose operation the plan lacks (`UnknownPicker`), for settings its tool's pickers do not read (`UnknownSetting`), for a picker not ready with the settings sent (`NotReady`), and once the pickers have failed (`SessionError::Pickers`).
- **It turns a picker on off first,** as `On` does, a one-shot still waiting for its reading among them, which then writes nothing but its turning on: a second one-shot sent before the first's reading arrives is the one read.
- **Ended early** as any picker is ([Turning off](#turning-off)): `Off`, another photo, a close, a second one-shot, or a command on the edit naming its tool before its reading arrives ends it, what it turned on a step of its own, as a picker's gesture under way is. Undo, Redo, a `GoTo`, Compress, Truncate and Discard History end it too, before they act, so its reading never writes over the edit they leave. Any other command on the edit before then makes its turning on a step of its own too, before that command's and never taken into it; the one-shot stays on, its pick a step of its own where it writes once its reading arrives.

**Against the live picker:** `On`, a click or drag at the same place and `Off`, as the button or the cursor and the photo send them, read the same region, a click's at Fit (decision 105), the values written those of the reading; the button's way differs in two cases, its first reading at the picker's kept place, applied before the click (Exposure's black level among what it writes), and a click on a kept area's corner, which moves that corner instead of starting a new area.

## The tool side

### Declaring pickers

A tool's definition lists its pickers, each darktable's `dt_color_picker_new` call:

```rust
pub struct Picker {
    /// snake_case and unique among the module's pickers, a namespace of their own: one on
    /// a slider's quad takes its control's id, as Exposure's `exposure`.
    pub id: &'static str,
    /// The operation whose input, and with `output` output, it reads: one the module owns
    /// a parameter of.
    pub operation: &'static str,
    /// darktable's POINT, AREA or POINT_AREA.
    pub kind: PickerKind,
    /// DT_COLOR_PICKER_DENOISE.
    pub denoise: bool,
    /// DT_COLOR_PICKER_IO.
    pub output: bool,
    /// darktable's picker_cst.
    pub space: Space,
    /// darktable's "keep-active": stays on when its module changes.
    pub keep: bool,
    /// The controls its pick may write; empty for one that only shows.
    pub writes: &'static [&'static str],
    /// The module's settings its pick reads.
    pub settings: &'static [&'static str],
    /// The choice setting whose option chooses its tooltip and its hint over the photo.
    pub words_by: Option<&'static str>,
}

pub enum PickerKind { Point, Area, PointArea }
pub enum Space { Operation, Lch, Hsl, Jzczhz }
```

- **`ProcessingDefinition::pickers`** holds them. `ProcessingDefinition::check` holds each to its tool: `writes` among its controls and `settings` among its settings, each once, `words_by` a choice setting of the tool, and each picker in exactly one place, a row or a drawn preset row's option that turns it on, a row of pickers holding at least one. `check::accept`, which knows the operations, holds the rest: the operation one the tool owns a parameter of, and a mosaic, the raw stage's, read only in `Space::Operation` with no denoise (darktable's denoise is 4-channel only).
- **Where its button sits,** by its row:
  - **`Quad::Picker(id)`** on a slider row, and on a choice row once Filmic RGB brings one ([Later tools](#later-tools)): darktable's quad (`dt_color_picker_new` on a bauhaus widget, `color_picker_proxy.c:411-427`), its tooltip darktable's quad tooltip, "pick color from image" where the tool sets none;
  - **`Row::Pickers { label, pickers, readout }`**, one line: an optional label at the start, or a readout in its place, and the picker buttons at the row's end, darktable's toggle buttons packed in a box (`:394-410`): AgX's Auto tune levels, RGB levels' black, grey and white, RGB curve's two, Colour calibration's readout row. A readout (`ReadoutDef { id, texts, decimals, swatch }`) is a text and a swatch the tool computes from its values, `ProcessingTool::readout`, pure as `view` is, giving one of its texts with its number and the swatch's colour (`pick::Readout`), which the session reports with the tool's state (`ToolState::readouts`) and the app draws, the swatch filling the row up to the buttons; its words are `tool-<tool>-readout-<id>-<text>`, its number `$value`, with `-promise`, and the swatch's `tool-<tool>-readout-<id>-swatch`. `ProcessingDefinition::check` holds the row to no label beside a readout, a readout to texts each named once, and its id to be unique among the tool's readouts.
  - **`PresetRow::pickers`**, an option of a preset row that turns a picker on instead of writing values, as an option id and a picker id: White balance's Settings' `spot`, from image area, as darktable's `_preset_tune_callback` presses the picker's button (`temperature.c:1816-1822`). `ProcessingDefinition::check` holds the option to be the row's, once, and the picker to be the tool's, and counts the option as the picker's place while the row is drawn. The session lists the option as available wherever the photo can be edited (`PresetState::available`) and `ApplyPreset` refuses it, `ProcessingTool::preset` giving `None`; the app sends `On` as an area for it, nothing while that picker is on, and draws it disabled where the pickers cannot be pressed. A script sends `PickerCommand::On` itself.
- **Its words by a setting** (`Picker::words_by`, decision 88): where a pick does something different by a setting's option, its tooltip and its hint over the photo say what, `tool-<tool>-picker-<picker>-promise-<option>` and `-hint-<option>` in place of `-promise`, each of the tool's number settings a variable of its id in the language's numbers, its decimals only where it has them: Exposure's "Make an area middle grey", "Make an area L 92 %". The viewer bar names the picker by its hint, "Exposure · Make an area middle grey". Each option of the setting has a tooltip of its own in the list, `tool-<tool>-setting-<setting>-option-<option>-promise` (`Choice::option_tips`).
- **`Row::Patches { picker, target }`:** darktable's input and target patches under an area mapping, Colour calibration's, side by side under their headings: the input the area the picker's last pick showed (`Shown::patch`), with its L readout, or for a target of three settings its L, h and c a line each, darktable's (`channelmixerrgb.c:4182-4191`), "N/A" before one and again once the tool's values change from outside a pick, as darktable's `gui_update` zeroes `spot_RGB` (`exposure.c:707-710`): undo, redo, a move through history, the edit's or the tool's reset, another photo; the target the colour of the settings `target` names, its L, then its C and h, painted as darktable's `_target_color_draw` paints it (`Patch::of_lch`), its tooltip the first setting's promise. `ProcessingDefinition::check` holds the picker to the tool's and the target to one to three numbers among its settings. Drawn disabled until the tool's pickers are drawn.
- **`ProcessingTool::pick`,** a provided method that gives nothing for a picker the tool does not run:

```rust
fn pick(
    &self,
    picker: &str,
    reading: &Reading,           // input and, with `output`, output Stats
    values: &Controls,           // every control, the edit's over the defaults
    image: &ImageContext,        // the camera's matrices and the pipe's profiles
    settings: &[(&str, ControlValue)],
    channel: Option<&str>,       // the channel tab the module shows
    last: Option<&Reading>,      // the module's last reading applied
    keys: PickKeys,
) -> Option<Picked>;

pub struct Reading {
    pub input: Stats,
    /// With the picker's `output` only.
    pub output: Option<Stats>,
    /// Each channel of the operation's input where the sensor saturated, the session's.
    pub input_maximum: [f32; 3],
}

pub struct Picked {
    /// Each control it writes with its value, within `writes`.
    pub writes: Controls,
    /// What its panel shows of the reading.
    pub shown: Shown,
}

pub struct Shown {
    /// The input patch.
    pub patch: Option<Patch>,
    /// The module's settings a pick measured, as a measure mode sets them.
    pub settings: Vec<(&'static str, ControlValue)>,
    /// What the pick did: the area's L before and after, the change, two patches.
    pub last: Option<LastPick>,
    /// Clipped pixels, and a pick held short of its target.
    pub warnings: PickWarnings,
}

pub struct Patch {
    /// Its colour for the screen: sRGB, encoded, 0 to 1.
    pub srgb: [f32; 3],
    /// Its L, C and h: CIE LCh of Lab D50, h in degrees.
    pub lch: [f32; 3],
}
```

- **A reading** always holds the input: a region that misses the operation's image measures nothing ([Events](#events), `ToolReading::input`), and nothing is picked from it.
- **The settings' values** are `ControlValue`s, as a `Row::Setting`'s kind is a control's.

Pure, as `bind` and `follow` are, so the tool's tests drive it with readings and no GPU. **The tool's last reading** is RGB levels' `last_picked_color` (`rgblevels.c:80`, `:788-796`), one per module, shared by its pickers: the session keeps each tool's last reading applied, for the photo, cleared as a photo opens, and passes it as `last`. Apart from it the session keeps darktable's per-module `picked_color`, the latest reading measured, applied or not, which `Settings` applies again ([What a pick writes](#what-a-pick-writes)). The tool's follow writes (decision 27) do not run after a pick: a pick writes what darktable's `color_picker_apply` writes, every control it keeps consistent included, as a preset row's option does.

**Built with Exposure's redesign** (decision 88):

- **`ProcessingTool::picker_ready(picker, settings) -> bool`,** provided as true, from the tool's settings alone: whether the picker can start. The app draws one not ready disabled, its `-unavailable` tooltip saying why (`messages::picker_unavailable`), and the session refuses its `On` (`SessionError::Refused`, reason `NotReady`), so a script cannot start it either; a tool that panics there is taken as not ready. A `Settings` that leaves the picker that is on not ready, as Exposure's Clear Reference while its picker is on toward the reference, applies nothing and turns it off, reported as any picker turning off is; its button is then drawn disabled with its reason, and the next `On` is refused with `NotReady`.
- **`Row::Reference { picker, lightness, photo }`,** a reference kept in settings ([A tool's settings](#a-tools-settings)): its picker writes no control (`ProcessingDefinition::check`, `ReferenceWrites`), it joins `Row::pickers()`, and the settings it names are the tool's, which a picker may read. Its picker's words include `-hint`, the viewer bar's name of it, "Exposure · Drag over the area to keep as the reference" (`messages::picker_hint`), and its row's words are shared (`messages::REFERENCE`).
- **`Row::LastPick { picker }`,** the last pick's readout ([gui.md](gui.md#the-rows)), naming a picker of the tool that writes controls, once (`ProcessingDefinition::check`, `LastPick`); its words are shared (`messages::LAST_PICK`) but for the picker's own on a black moved (`messages::short_black`).
- **`Shown`** has `last: Option<LastPick { before, after, exposure, patches: [Patch; 2] }>`, the area's L before and after the pick and the change of the control in its unit, and `warnings: PickWarnings { clipped: bool, short: Option<Short { exposure, reached, target, black }> }`, independent, so a pick can give both; `black` says the pick moved the black too, which the short warning's tooltip adds. The app keeps both per tool as it keeps the patch, cleared by the same commands, and for a picker with a `Row::LastPick` the live region says its words, its warnings first, in place of `picker-said-control` (`pickers::LastPickWords`).
- **`Reading`** has `input_maximum: [f32; 3]`, what each of red, green and blue of the operation's input reaches where the sensor saturated (`ImageDesc::maximum`), which the session fills in from the plan as it measures.

### The first pass's pickers

Each tool's picker, as the reference release declares and applies it. Every one reads its operation's input in the operation's own space unless the table says otherwise.

| Tool | Picker id | darktable's widget and kind | Writes, from the reading | Source |
| --- | --- | --- | --- | --- |
| Exposure | `exposure` | Quad on Exposure; area | Correction: Exposure `−log2(Y / Y_target)` of the mean's luminance Y, through the input profile's matrix to XYZ and made grey of its L, against the grey of the target lightness, plus the camera's bias where Compensate camera exposure is on, less the held-back exposure where Highlight preservation mode is on, which the operation takes off and gives back again, unbounded by darktable; nothing written where it equals the stored value; Black, where the black the operation holds is at or above the new white `2^−e`, set to white − 0.01 (`_exposure_set_white`, `:763-779`), written as Black level correction over the photo's own black. The target lightness is Picker target's: L 50 for Middle grey, the Lightness setting for Lightness, the reference's L for Reference, the picker not ready with none set (decision 88). darktable's measure mode is gone, to the `reference` picker. The patch: the area as a grey of its L. Exposure's pick fills Last pick, before and after by the operation's arithmetic with its black, and its warnings, `clipped` at 0.99 × `input_maximum` in any one channel and `short` held at ±18 EV (decision 88) | `exposure.c:866-973`, `:1211-1218`, `:763-779` |
| Exposure (decision 88) | `reference` | The Reference row's picker; area | None: measure mode's lightness of the area at the current exposure, held to 0 to 100, handed back as `reference_lightness` | `exposure.c:900-934` |
| White balance | `spot` | A toggle in the button bar, and Settings' "from image area"; area; space none, so the mosaic per CFA colour | Each coefficient `(1 / mean) / (1 / green's mean)`, held by darktable's own rule to 0 to 8, a mean of 0.001 or below taken as giving 1, green 1; `preset` from image area. The area comes out neutral only while every mean is above that floor and every coefficient inside 0 to 8 | `temperature.c:1948-1969`, `:2082-2095`, `:1816-1822` |
| Colour calibration | `illuminant` | A toggle on the CAT tab's readout row; area | Correction: the illuminant `custom`, its x and y `D50 ∘ LMS / LMS_target` in the adaptation's LMS, each Y-normalised, the target first through the inverse mixer with Take channel mixing into account (`:4286-4410`), and the temperature `_check_if_close_to_daylight` gives for them; no bound of darktable's own. Measure: the target L, h and c the area gives. The patch with its L, h and c | `channelmixerrgb.c:4161-4421`, `:4486-4489` |
| Colour balance RGB | `global_hue`, `shadows_hue`, `midtones_hue`, `highlights_hue` | Quads on the four hues; area | The hue the opposite of the area's, its own with `command`: `atan2` of the mean's Ych in degrees plus the tool's 30° shift, plus 180° for the opposite, so −150° to 390°, which darktable stores as it is and Apertrust wraps onto 0° to 360°; the chroma the mean's chroma times its luminance, unbounded by darktable. Ych through the working profile | `colorbalancergb.c:1278-1338`, `:1875-1953` |
| Colour balance RGB | `white_fulcrum`, `grey_fulcrum` | Quads; area | White fulcrum `log2` of the maximum's luminance; contrast grey fulcrum the mean's luminance; neither bounded by darktable | `colorbalancergb.c:1324-1333`, `:2019-2024` |
| AgX | `white`, `black` | Quads on white and black relative exposure; area, denoised | `log2(max(ε, v) / 0.18)` of the channels' largest maximum, or of the smallest minimum, floored at AgX's `_epsilon` first (`agx.c:1110`, `:1126`), scaled by 1 + Dynamic range scaling, held by darktable's own rule to −20 to −0.1 EV for black and 0.1 to 20 EV for white; then the pivot kept at its exposure, held within the new range (`_update_pivot_x`); Curve y gamma left alone, as Keep the pivot on the diagonal leaves it (`agx.c:2657-2690`; `docs/tools/agx.md`, deliberate difference 3; decision 63) | `agx.c:1103-1133`, `:1021-1040`, `:2043-2057` |
| AgX | `range` | Auto tune levels, a toggle on a row of its own; area, denoised | Both at once; Curve y gamma left alone, as above | `agx.c:1135-1160`, `:2072` |
| AgX | `pivot`, `pivot_output` | Quads on Pivot relative exposure and Pivot target output; area, denoised | The pivot at `log2(max(ε, Y) / 0.18)` of the mean's luminance through the working profile, floored at `_epsilon` (`:1178`), held by darktable's rule within black and white relative exposure; with Pivot target output, also the output the curve gave that pivot before, linearised by the curve's gamma, so its brightness stays; Curve y gamma left alone, as above | `agx.c:1171-1220`, `:1823-1840` |
| RGB curve | `show` | A toggle beside the channel tabs; point or area; output too; keep-active | None: the graph draws the input's and the output's mean, minimum and maximum, and the stored samples | `rgbcurve.c:1496-1503`, `:930-1035` |
| RGB curve | `create` | A toggle; area; output too | The shown channel's curve reset, then nodes at the minimum, the mean and the maximum, the mean moved 0.05 up with `command` or down with `shift`; where that makes five nodes, a fourth added midway between the second and the fourth, moved the same way (`:573-578`) | `rgbcurve.c:534-583`, `:1505-1518` |
| RGB levels | `black`, `grey`, `white` | Three toggles in a row; point | For the shown channel, the first channel's mean (`*self->picked_color`, `:786`), applied only when it differs from the tool's last one picked (`last`, `:788-796`): black takes it unless it is above grey, then grey − `FLT_EPSILON`; grey takes it only if it lies between black and white, otherwise it is left unchanged; white takes it unless it is below grey, then grey + `FLT_EPSILON`; a step only when a point changed (`:832-837`) | `rgblevels.c:774-838`, `:1068-1077` |

- **AgX's Dynamic range scaling** is the pickers' margin only (`docs/tools/agx.md`, Gaps 2; decision 63): it scales the range a pick writes and writes nothing itself.
- **White balance's Settings** shows "from image area" after a pick, as darktable's combobox shows it at once (`temperature.c:1967`), its stored `preset` `spot`; choosing "from image area" turns the picker on, as darktable's `_preset_tune_callback` does (`temperature.c:1816-1822`). Only in the session it was picked: whenever history's saved state is read again, as darktable redraws the module, as the photo opens and after Undo, Redo or a `GoTo` among them, the label is worked out from the coefficients as darktable's `gui_update` does (`temperature.c:1195-1360`), and a picked balance is "user modified" (`:1338`), unless its coefficients match as shot or camera reference within darktable's 1e-5, which it then reads, the stored `preset` too, in the session's edit, saved only with White balance's own next change, never a step of its own, nothing written on open (`docs/tools/white-balance.md`, After a pick).
- **RGB levels' picked point** is stored by darktable's rule above, with no other bound than its parameter's 0 to 1, as `docs/ports/rgblevels.md`'s Tool binding notes state.

### Later tools

| Tool | Pickers | Brings |
| --- | --- | --- |
| Filmic RGB | Grey, white and black of the scene, and Auto tune levels on a choice row's quad; area, denoised (`filmicrgb.c:4365-4420`, `:2701-2714`) | A quad on a choice row |
| Colour equalizer | The hue shift's quad, point or area, denoised, in JzCzhz, keep-active, showing the hue on the graph; White level's, area (`colorequal.c:3003-3012`, `:3094`, `:2541-2560`) | `Space::Jzczhz` |
| Blending (Phase 4) | Show colour, point or area, output too, keep-active, and set the range from the area, each in the blend space (`blend_gui.c:2578-2595`, `:1757`) | `Space::Lch` and `Space::Hsl`; a layer's instance in `PickerRef` |
| Colour calibration's colour checker | Its patches' means through the chart's frame | Many boxes in one reading |

**Layers** (Phase 4) add the instance to `PickerRef::Tool`: a layer and an instance, as history's entries gain them, the reading taken at that instance's place.

## The picker button

A component of `apertrust-ui`, `PickerButton`, an `IconButton` toggle that draws no engine type: the renderer hands it whether it is on and enabled, and it reports what was pressed.

- **Its look:** Lucide's `pipette`, 18 by 18 in a slider or choice row (before the value or the option, gui.md, How the rows are laid out) and in a `Row::Pickers`; RGB curve's create-curve picker draws darktable's set-values shape (`dtgtk_cairo_paint_colorpicker_set_values`), a pipette with a plus at its top left, drawn for Apertrust in Lucide's style, Lucide's `pipette` with a `plus`.
- **Its states:** an icon button's, with no plate (decision 81; gui.md, Checkboxes, text buttons and icon buttons): off, the outlined pipette in secondary text; under the pointer, the same in `text_strong`; held down, in `accent_text`; on, while picking, the filled pipette (`pick-filled`) in `text_strong`; disabled, dimmed, on a photo not yet live and, for a tool's picker, on a photo open for viewing only, with the tooltip saying why; focused, the focus ring.
- **One of two doors** (built, decision 104's 104.5 and 104.6): the picker's cursor in the Toolbar's well is the other ([cursors.md](cursors.md)). Both send the same `On`; the button is on and the well's face chosen exactly while the picker is on, and either turns it off. The button keeps its own look while on, the filled pipette in `text_strong`: only the well's cursors are drawn in the accent.
- **Its tooltip** is darktable's words for it ("set the exposure adjustment using the selected area", "pick opposite color from image / ctrl+click to pick selected color"), ending, for a picker with both kinds, with darktable's "ctrl+click or right-click to select an area", on a quad "ctrl+click to select an area" (`colorequal.c:3010-3011`), Cmd-click on the Mac. Where darktable's own tooltip already holds that line (`rgbcurve.c:1500-1501`, `libs/colorpicker.c:778`), the tool's message holds only its first line and the button adds the rest.

| The user | `PickerAction` | Command |
| --- | --- | --- |
| Clicks it while off | `On { area: false }` | `On`: a point, or the picker's one kind |
| Right-clicks a toggle button, or Ctrl-clicks either (Cmd-click on the Mac), Ctrl alone held | `On { area: true }` | `On` as an area where the picker has both |
| Right- or middle-clicks a quad | As a click | |
| Clicks it while on | `Off` | `Off` |
| Space or Enter while focused | As a click | |
| Shift+F10 or the Menu key while focused | `On { area: true }`, on a quad too | As above |

- **A toggle button and a quad** press as darktable's do. Its toggle button (`dtgtk_togglebutton`) takes a right-click as an area (`color_picker_proxy.c:178-179`); its quad takes every mouse button alike, the bauhaus quad calling back with no event (`:259-263`, `:422-424`), so there only Ctrl picks an area. Ctrl must be held alone, as `dt_modifier_is` compares the whole mask: Ctrl+Shift is a click. The keyboard's Shift+F10 and Menu key pick an area on both, as the app's keyboard way to one.

**Built** (79.5, `apertrust_ui::components`): `PickerButton::new(name, on)`, with `tip` (by default `picker-tip`, darktable's "pick color from image"), `point_or_area` (the tooltip's `picker-tip-area`, or `picker-tip-area-quad` on a quad, Ctrl or Cmd by the platform), `quad`, `disabled(why)`, `shortcut` and `state`; `show` gives its `Response` and a `PickerAction`, `On { area }` or `Off`. Any press while on is `Off`, a right-click, Shift+F10 or the Menu key included, as darktable's press on the active toggle button or quad (`color_picker_proxy.c:237-252`). It draws as an `IconButton` toggle (decision 81): on, the filled pipette (`pick-filled`) in `text_strong`, no plate; at rest, under the pointer, held down, focused and disabled, an icon button's states. To AccessKit it is a button with its toggled state, its tooltip its description, disabled with no focus while disabled.

**Built** (79.6, the app's `tools.rs`): a slider's `Quad::Picker(id)` is a `PickerButton` in its quad mode (`SliderRow::picker`, reported as `SliderAction::Picker`), and a `Row::Pickers` its label at the start and toggle `PickerButton`s at the row's end. Each is on while the mirror shows its picker on, its full name `tool-<tool>-picker-<picker>-full` and its tooltip `-promise` (`messages::picker`), disabled on a photo opening, saying "Opening IMG_2041…", and for a tool's picker on one open for viewing only, saying which version wrote its edit; disabled with the row where its slider is. A press sends `On` with the keys held, or `Off` for the picker on, and shows the picker on or off at once, before the session answers. The panel draws a tool's pickers only once its pick is built (`tools::PICKING`, each tool joining with its pick: Exposure's, 79.7), so a quad with nothing behind it is never drawn. `On` carries the tool's settings as the app keeps them, and no channel until a tool reads one.

On a Mac, Control-click is the right-click, as everywhere in the app (gui.md, Popups). Turning a picker on moves the keyboard focus to the photo, as darktable gives the centre view the focus (`lib.c:1590`, `:1598`); turned on from outside its section, by the Colour picker's Pick colour action, it is to open its tool's section if it is closed (planned with 79.16; a tool's own button sits in its open section). Turned on from the well, a tool's picker neither scrolls to nor opens its tool's section: the label on its area and the pointer's badge say whose it is (decision 104). The renderer gives every quad its picker's state from the mirror's `Picker` fact, and the app sends the settings the tool's pickers read with `On`.

## On the photo

The Viewer draws the active picker over the frame, and the stored samples once they are built (planned, steps 79.14 and 79.16), and turns the pointer into commands, from the regions in the photo's coordinates that the events carry. Every mark is drawn with the design system's on-photo overlay, one line and two handles for every mark on the photo ([gui.md, On-photo overlays](gui.md#on-photo-overlays)), as the wireframe's Pickers frame draws them; the app owns the commands.

**The picker's marks,** darktable's `_darkroom_pickers_draw` (`darkroom.c:331-470`) in that language:

| Mark | Drawn as |
| --- | --- |
| The active picker's area | Its rectangle, the normal weight, with a square handle on each corner, stroked |
| A stored area | Its rectangle, secondary, dashed 4 on 4 |
| The active picker's point | A circle of radius 4 half-pixels, 16 points at Fit, with a crosshair through it of the same reach; at its centre its sampled colour |
| A stored point | The crosshair alone, its arms 5 half-pixels, 20 points at Fit, with its sampled colour at its centre |
| The sample under the pointer in the Colour picker's list | Its lines twice as thick; a point's crosshair twice as large and its centre a disc of twice the half-pixel; an area the same size, solid rather than dashed |

- **Sizes are in points,** darktable's logical pixels, as the overlay's are ([gui.md](gui.md#on-photo-overlays)).
- **The half-pixel** is half a frame pixel measured in points, rounded, never less than 4 points, as darktable's `half_px`, so the marks keep their size at Fit and grow only once a frame pixel spans 9 points or more.
- **The centre,** darktable's rule (`darkroom.c:458-465`): while a frame pixel spans under 7 points, its half rounding below 4, a disc of radius 4 points; from 7 points on, a square one frame pixel wide whose top edge runs through the point, showing the very pixel sampled; for the sample under the pointer, a disc of twice the half-pixel.
- **The sampled colour** is the sample's frame mean, or the statistic the Colour picker shows: with the mask's display colour, the only colour the overlay puts on the photo.
- **When:** the active picker while it is on; the stored samples while Show samples on the photo is on, else only the one under the pointer in the list (`darkroom.c:801-829`). Nothing is drawn while the photo is not live (decision 77).
- **Built** (79.5): `PickerMarks::new(PickerPlace)`, `Point { at, colour }` or `Area(rect)` in the photo's coordinates, `focused`, and `paint` on the photo's `Overlay`: the active area and the active point as the table draws them, and focused, the overlay's ring round the area with its handles or round the point's reach (`Overlay::focus_rect`, `focus_circle`). Stored samples' marks come with them (79.14, 79.16).
- **The label on the area** (built, 104.7): the active area or point carries a small label naming its tool, "White balance", "Exposure", and where the tool has more than one picker a person can turn on, "Tool · picker" as the viewer bar's `picker-bar-name` gives it, "Exposure · Reference", but where the picker's name, in the words shown, repeats the tool's, it once: Exposure's own picker's label is "Exposure", not "Exposure · Exposure"; a Fluent message of its own, `picker-area-label`, with the same arguments. It is `Overlay::label`'s small text in `overlay_line` over `overlay_halo`, with no interface colour. For an area it sits outside it, above the top-left corner handle, left-aligned with the handle's outer edge and 4 points above it, or 4 points above the focus ring while the area has the focus; for a point, 4 points right of its reach, centred on it. Near the edge of the part shown it moves below the bottom-left handle, right-aligns to the right-hand handles, or for a point flips to the left; where none fits, it goes inside the top-left corner, 6 points past the handle, kept inside the part shown where that corner is past its edge. It follows the area as dragged, is no target for a press, is not drawn while the photo is not live or the area lies wholly outside the part shown, and is decorative to screen readers, the area's node carrying the name.
- **The pointer's badge** (built, 104.5 and 104.7): over the photo while a picker is on, the system crosshair with the picker's cursor icon beside it, 12 points, 12 points below and right of the pointer's hotspot, stroked in `overlay_line` over `overlay_halo` and flipped to the other side near the photo's right or bottom edge; hidden while the area is dragged, the label then doing its work. The overlay draws it (`Overlay::icon`), not a system custom pointer, so it trails the hardware pointer by a frame.
- **The viewer bar,** while a picker is on, says which picker it is and how to use it, as the wireframe's Pickers frame has it; the tool's row shows the patch with what was measured (`Shown::patch`).

**The viewer bar's words,** Fluent messages in `ui.ftl`, Ctrl written Cmd on the Mac by the app's modifier rule:

| Part | Message | British English |
| --- | --- | --- |
| Its start: the tool and the control or picker | `picker-bar-name` | "{ $tool } · { $picker }": "Colour calibration · Illuminant", "RGB levels · Grey point" |
| An area picker | `picker-bar-area` | "Click for a small area, or drag one. Drag a corner to resize. Right-click resets the area. Esc ends." |
| A point picker, after its own sentence | `picker-bar-point` | "{ $what } Esc ends." |
| A picker with both kinds, measuring a point | `picker-bar-both-point` | "Click for a point, drag for an area. Esc ends." |
| A picker with both kinds, measuring an area | `picker-bar-both-area` | "Click for a point, drag for an area. Drag a corner to resize. Right-click resets the area. Esc ends." |

- **The words follow the picker's declared kinds** (decision 105, planned: the area picker's with 105.2, the both kinds' with 105.5). Built, an area "Drag an area on the photo. Ctrl-click switches to a point. Right-click resets the area. Esc ends.", a point "{ $what } Ctrl-click switches to an area. Esc ends.", and `picker-bar-point-what` for an area picker switched to a point, which goes, since no area picker is ever a point.

- **The picker's own sentence** names what to pick, as its row's tooltip does, as Colour calibration's "Pick the illuminant from a neutral area of the photo": a point picker's `tool-<tool>-picker-<picker>-what` message, as RGB levels' black, grey and white (`DT_COLOR_PICKER_POINT`, `rgblevels.c:1069`); the example is RGB levels' grey picker, "RGB levels · Grey point" with "Click or drag the point on a neutral grey." A picker with both kinds, as RGB curve's show picker and the Colour picker, says the both kinds' words, with no sentence of its own.
- **The picker's name** is its control's label where it sits in a quad ("Exposure"), else its row's or its button's words ("Illuminant", "Auto tune levels", "Grey point"), `tool-<tool>-picker-<picker>-label`.
- **The Colour picker's own picker** names "Colour picker" alone.

**The pointer,** darktable's `button_pressed`, `mouse_moved` and `button_released` (`darkroom.c:3815-3948`, `:3984-4140`), while a picker is on, by the kinds the picker declares (decision 105, planned; built, as the Built list below says):

- **A click or a drag.** A press is a click while the pointer stays within 6 points of where it went down, egui's `max_click_dist`, by distance alone however long it is held, never by egui's 0.8 s click time; past them it is a drag until the release, wherever the pointer goes after. Until then nothing follows the pointer: what the press sent stays. The threshold is in points on the screen at every zoom, never in the photo's pixels: it measures the hand, not the image.
- **The click's area** is the size on screen that darktable's area has at Fit, at every zoom: its half side across 2 % of the photo's width at Fit, in points, as tall on screen, round the press and clamped to the photo. At Fit it is `Region::pressed`'s area; zoomed in, it covers the same patch of the screen, not a share of the photo. The app works it out from the view and sends it as `Area`; `Once` keeps `Region::pressed`.

| The user | An area picker | A point picker | A picker with both kinds |
| --- | --- | --- | --- |
| Presses elsewhere than below and lets go within the threshold, a click | `Area`, the click's area round the press, `done: false` on the press, so the tool follows at once; the release `done: true` | `Point { at }` at the press, `done: false`; the release `done: true` | `Point` at the press, `done: false`; the release `done: true`; its area kept as the other kind's place |
| Presses elsewhere and drags past the threshold | `Area` from the press to the pointer, at least one frame pixel each way, each step `done: false`, the release `done: true` | The point follows the pointer, each step `done: false`, the release `done: true` | `Area` from the press to the pointer, as an area picker's |
| Presses within 6 points of a corner of the area shown | That corner taken, the opposite one held; past the threshold `Area` from the held corner to the pointer, the release `done: true`; let go within it, nothing sent | – | The same, while it measures an area |
| Presses on the point's mark, within its circle | – | As any press: the point there | While it measures a point, the point taken: past the threshold it follows the pointer, no area drawn; let go within it, nothing sent |
| Ctrl-clicks (Cmd-click on the Mac) | A click, `command` in its keys, read by the pick as darktable reads the modifier: Colour balance RGB's hue picker takes the colour itself, RGB curve's create curve makes a positive curve | A click, `command` in its keys | A click, `command` in its keys: no switch |
| Right-clicks, with Show samples on the photo on, on a stored sample of a kind the picker takes | `Load { id }`, within `max(26, round(3 × zoom))` points of a point sample, `zoom` the points a frame pixel spans, so the reach grows with the zoom (`darkroom.c:4108-4113`), or inside an area sample | The same | The same |
| Right-clicks otherwise | `Reset` | Nothing | `Reset` while it measures an area; nothing on a point |
| Holds a press still | Nothing more: egui's long press on a touch screen, a secondary click, is not taken as a right-click, so it never resets the area, until Touch mode settles touch (decision 94) | The same | The same |

- **A picker never changes kind on the photo:** an area picker is never a point and a point picker never an area, so White balance never reads one photosite; a picker with both kinds goes from one to the other by the gesture alone. Its button still chooses the kind it starts as ([The picker button](#the-picker-button)).
- **A stray click replaces the area** on a picker that writes, as in darktable: the area is drawn again by hand, and Undo takes back the values only, turning the picker off as any change of its tool does. A click moving the area and keeping its size is an [open question](#open-questions).

- **While a picker is on its cursor is the chosen one,** from either door ([cursors.md](cursors.md#a-cursor), decision 104), and the photo is the picker's. A right-click is the picker's too, and the photo's menu does not open then (gui.md, Context menus).
- The photo does not pan while a picker takes its drags; in a zoomed view, dragging past the view's edge scrolls it, as darktable's does (`darkroom.c:3895-3902`). Holding A borrows Pan over the picker, darktable's force-pan, and gives the photo back to it on release, the picker still on. The pointer is the crosshair with the picker's badge, the move pointer over a corner's handle and during an area's drag, the badge gone while A is held.
- **What is drawn follows the pointer at once:** the app draws the area or point it sent until a reading includes it, as the panel shows a pending value (gui.md, The renderer), so the drag never waits for a reading.

**Keys,** with the picker's point or area focused on the photo: Apertrust's own, darktable having none there, following the quality bar's "everything can be reached by keyboard". While its node has the focus, it takes these keys before the app's actions, so the arrows, Space and Backspace move the picker rather than going to the next or previous photo (gui.md, Next Photo and Previous Photo; Who owns the keys):

| Key | Does |
| --- | --- |
| Arrow keys | Move the point, or the whole area, one frame pixel; Shift ten |
| Alt+arrow keys | Move an area's right or bottom edge one frame pixel, growing or shrinking it; Shift+Alt ten |
| Space | Nothing: no switch on the photo (decision 105, planned; built, `Switch`). The node keeps it, so Space never goes to the next photo while picking |
| Delete or Backspace | `Reset`, the picker back to its default point or area |
| Esc | `Off`, which chooses Select in the well; the focus back to the picker's button, or to the well's face where the well turned it on |

Each press is a completed change, so a held arrow's repeats merge into one step, as a slider's do (decision 78). **Esc ends the picker wherever the focus is**, not only while its node has it: a field being typed in or an open list keeps the first Esc, the next turning the picker off.

**Built** (79.6, the app's `pickers.rs`):

- **The marks** are `PickerMarks` on the photo's overlay at the frame's zoom, the region the reading gives or, until a reading includes it, the one the app sent; nothing before a picker's first reading, and nothing while the photo is not live. A point's centre is the reading's frame mean.
- **The pointer's table** as above, a corner taken where the press is strictly within 6 points of the area's nearest edge on each axis, as darktable's `MIN(dx0, dx1) < hx` (`darkroom.c:4050-4062`); on the Mac, Control-click is the right-click, resetting an area, and Cmd-click switches; Ctrl-click (Cmd-click) and Space switch any picker, whatever its kinds; any move after a press, with no threshold, starts the drag, and egui's long press on a touch screen resets an area (each to change with decision 105: 105.1 to 105.5); a new area's half side 2 % of the photo's width, as tall on screen, clamped to the photo, at every zoom, the drag from the press at least a frame pixel each way; a corner taken and let go where it was sends nothing; a right-click on a point does nothing. Stored samples' `Load` comes with them (79.14).
- **The keys' table** as above, while the picker's node on the photo has the keyboard's focus: the focused widget keeps the arrows, Space, Delete, Backspace and Esc from the app's actions (gui.md, Who owns the keys), and egui moves no focus on them. A move holds the area inside the photo at its size; a resize keeps it at least a frame pixel; Alt with the arrows on a point does nothing.
- **Focus:** turning a picker on moves the focus to its node on the photo, once its first reading shows it; a press on the photo takes the focus too; Esc gives it back to the picker's button.
- **The label and the badge** (104.7): `pickers::label`, the tool's title, or `picker-area-label` where the tool declares more than one picker, a preset row's included, and the picker's name is not the title, and `pickers::badge`, its cursor's icon or the pipette, drawn by `on_photo` with `Overlay::mark_label` and `Overlay::badge` at the region shown, the badge while the pointer is over the photo area, hidden while an area is dragged.
- **The viewer bar's words** take the place of the values under the cursor ([gui.md](gui.md#the-viewer-bar)), the built ones as the note under the table above has them, how left out where it does not fit; the Colour picker's come with the Colour picker tool (79.16).

## Accessibility

- **The button** is a toggle button to AccessKit, named by its action, its full name, or where its words follow a setting what a pick will do ("Make an area middle grey", "Match an area to IMG_2041 (L 62 %)"; a picker that cannot start, why not), pressed while on, its keys as its shortcuts, its tooltip its description.
- **The picker on the photo** is a node of its own while a picker is on, a slider-like group named "Picker area, Exposure" or "Picker point, Colour picker", its value the region in percent of the photo ("point at 52%, 40%", "area from 10%, 12% to 30%, 25%"), with Increment and Decrement moving it along x as → and ← do, the custom actions Move up and Move down moving it along y, and its keys as described. The custom actions are in the AccessKit tree but reach no screen reader until AccessKit's platform adapters pass them on (0.25's do not); ↑ and ↓ move it meanwhile.
- **Turning on and off is said** politely: "White balance picker on" as it turns on, from either door; as it turns off where the person did not ask, the reason, "White balance picker off: Temperature changed", "Exposure picker off: another photo", which the app names from what turned it off, a reason lasting only until the session answers the command it came from; another picker replacing it, both in one announcement; the well, the pointer, the area and its label change in the same frame.
- **The reading is read out** through a polite live region, once a reading follows a move or a pick and the pointer or the keys have rested half a second: the Colour picker's value in its colour mode ("mean RGB 128, 120, 99"), and for a tool's pick what it changed ("Exposure +0.84 EV"), from the `Values` that follow.
- **The Colour picker's rows** are labelled rows: each sample's patch a toggle button "Lock sample 2", its value text read with its mode, its remove button "Remove sample 2", the number its place in the list ([Commands](#commands), Ids and numbers).
- **Colour is never the only cue:** the swatches carry their values as text, and the area's lines are drawn light over dark, visible on any photo.
- **Built** (79.6): the button and the picker on the photo as above; the live region a polite status, saying each control the pick changed against the values as its gesture began, with its value as its row writes it ("Exposure +0.84 EV", `picker-said-control`), and where it changed none the frame's mean as RGB 0 to 255 ("mean RGB 128, 120, 99", `picker-said-rgb`), once a reading through the last move or press has come and the picker has rested half a second.

## The Colour picker

darktable's color picker utility (`src/libs/colorpicker.c`, in the darkroom's left panel with its history and snapshots, position 800), as the wireframe's Pickers frame has it: a utility tool of the Tools panel, hidden by default, in no tab of Edit, so in the Hidden tab and found by Find Tool… (decision 96), its tab to be decided, closed when shown and kept open or closed with the other sections (decision 38). Its rows, darktable's (`:696-865`) in the frame's order and words, top to bottom:

1. **The header,** the section's, with the picker button in it, one of the picker's two doors, the other the Colour picker's own group in the Toolbar's well, on Shift+K (decision 104's 104.8; [cursors.md](cursors.md#the-groups)): darktable's picker button, its kind chosen as darktable's is, a click for a point, right-click or Ctrl-click (Cmd-click on the Mac) for an area (`:775-779`), its tooltip darktable's but for its last line: "Turn on colour picker / Ctrl+click or right-click to select an area / Click the photo for a point, drag for an area" (decision 105). With it, a Point | Area choice for the live sample, the keyboard's way to the other kind, sending `Switch` while the picker is on (decision 105, with 79.16; whether it and the well's Start as are one setting is settled with the tool).
2. **One row of two choices:** the statistic, Mean, Min or Max, then the colour mode, darktable's eight: RGB, RGB %, Lab, LCh, HSL, HSV, Hex and None (`:754-773`).
3. **The live sample:** a patch 52 points square and its values beside it, one channel per line, in IBM Plex Mono and selectable (`:786-815`). A click on the patch shows and hides darktable's large patch above the rows, hidden by default (`ui_last/colorpicker_large`, `:272-283`).
4. **Add sample,** a plate button (`Button`), enabled only while a picker is on (`:816-821`, `:268-269`).
5. **The stored samples,** numbered by their place in the list, from 1, one row each: its number, a small patch (a click locks it, a lock drawn on it; a right-click loads it into the active picker, or stores the picker's place into it, a store mark drawn on it while loaded), its values and × to remove it, in a `section_list` (`:597-673`).
6. **Two checkboxes:** Show samples on the photo, darktable's "display samples on image/vectorscope" (`:830-840`), and Restrict the scopes to the selection, darktable's "restrict scope to selection" (`:842-853`).

- **Values,** darktable's `_update_sample_label` (`:188-259`) from the reading's statistic: RGB, each channel of `rgb` times 255 rounded; RGB %, times 100 with one decimal; Lab, L clamped to 0 to 100 and two decimals; LCh, from Lab, h in degrees; HSL and HSV, from `rgb`, hue in degrees and the rest in percent; Hex, `0x` and the three bytes clamped; None, "◎". The patch is the statistic's `rgb` as the screen shows it.
- **The tooltip** of a value, darktable's (`:356-425`): a table of mean, min and max as RGB with their swatches and hex, Lab, and the colour's name from the mean's LCh (`Lch_to_color_name`, `src/common/color_vocabulary.c`, restated in `apertrust-color`).
- **Hovering a sample's row** marks it on the photo, as darktable's `selected_sample` (`:476-506`).
- **Restrict the scopes to the selection** makes the scopes count only the Colour picker's point or area while its picker is on, as darktable's histogram does (`src/libs/histogram.c:115-160`): `Command::Scopes` gains the restriction, and the scopes' kernel counts within the frame's rectangle it maps to. With it on, another tool's control changing or another tool's Show on photo turning on leaves the Colour picker's picker on ([Turning off](#turning-off)).
- **Show samples on the photo** also marks each stored sample's mean on the vectorscope, as darktable's checkbox does (`src/libs/scopes/vectorscope.c:622-640`), placed on the plane by the scopes' own `Plane` (decision 19), and RGB curve's graph draws the stored samples, as `rgbcurve.c:936-990` does.
- **Its reset** in the header sends `SamplesCommand::Clear` and sets Mean, RGB and Show samples off (`gui_reset`).
- **Its settings,** darktable's `ui_last/colorpicker_*`: the statistic, the colour mode, the large patch, Show samples and Restrict the scopes, in `settings.toml` under `[colour_picker]`, declared in the settings schema; the stored samples last only for the session, as darktable's do.
- **Actions,** darktable's: Pick colour (the button, a toggle), Add sample, Show samples and Restrict the scopes, each bindable in Settings › Shortcuts, with no default keys, as darktable has none.

## Against darktable

| darktable 5.6.2 | Apertrust | Why |
| --- | --- | --- |
| Measured in the preview pipe, downscaled throughout, the mosaic included | In the fitted viewport pipe: after the downscale at the viewport's size, in the raw stage at full resolution | Decision 63: the export's processing at the view's scale; White balance's mean is the photosites the export reads |
| Measured on the CPU, or copied from the GPU to the host for it (`_pixelpipe_picker_cl`, `pixelpipe_hb.c:1008-1106`) | Reduced on the GPU, 48 bytes a region read back | GPU-resident pipeline (decision 8) |
| A picker's reading applied by the GUI's callback | Applied by the session through `ProcessingTool::pick`, the same for every client | Every client through the session protocol (decision 58) |
| Each applied reading its own undo record (`dt_dev_add_history_item` with no target) | A drag's readings one gesture, one step of source Auto, merged as a control's | Decision 78 |
| No keys on the photo for a picker | With its point or area focused: arrows move it, Alt+arrows resize an area, Delete resets it, Esc turns it off; the region a node for screen readers | The quality bar: everything can be reached by keyboard |
| Ctrl-click on the photo switches every picker between point and area, whatever kinds it declares (`darkroom.c:4017-4034`) | No switch on the photo (decision 105, planned): a picker that takes one kind keeps it, a Ctrl-click a plain click carrying `command`, and the session refuses `Switch`, and a `Point` or `Area` of the other kind, for it | darktable's manual limits the switch to "modules where both modes are available"; a point on an area picker reads one photosite of the mosaic, or one pixel's noise |
| On a picker with both kinds, the kind kept until a Ctrl-click switches it | A click picks a point and a drag draws an area (decision 105, planned, with 79.16); the Colour picker tool's Point \| Area choice the keyboard's way | An area, the right sample for skin, needs no hidden modifier; these pickers write nothing |
| Any motion after a press drags (`mouse_moved`, `darkroom.c:3843-3858`) | A click while the pointer stays within 6 points, by distance alone; egui's long press is no right-click (decision 105, planned) | A click's jitter no longer replaces its area with one a pixel wide, nor a still finger resets it |
| A click's area 2 % of the photo's width each way, at every zoom (`darkroom.c:4064-4073`) | The same at Fit, and that size on screen at every zoom (decision 105, planned); `Once` keeps darktable's | Zoomed in, a click picks the patch pointed at, not a share of the photo past the view |
| The utility in the darkroom's left panel, with the history | A utility tool of the Tools panel, hidden by default, its place to be decided | One Tools panel (decision 38) |
| A picked value stored as its tool's rule gives it, past a parameter's `$MIN` or `$MAX` where the rule has no bound | Held at the nearer hard limit | Every stored value within its parameter's range (decisions 21 and 30) |
| Colour balance RGB's picked hue stored from −150° to 390° | Wrapped onto 0° to 360°, the same angle | Within the hue's hard limits, the render unchanged |
| A reading applied only where the point or box differs from the last applied (`_record_point_area`, `color_picker_proxy.c:76-104`), so a Ctrl-click to the same box applies nothing | `Switch` always applies the reading of the kind it switches to | Point and area are each kept in the original image's coordinates, so the other kind's place is always another region; a client never needs to know which differs |
| The box dragged on the photo padded by one full pipe's pixel on each side (`darkroom.c:3844-3858`) | At least one frame pixel each way, in the photo's coordinates | The app draws the fitted frame and knows no other pipe's pixels; the session maps the region to each buffer's pixels and makes every box at least one pixel ([Regions](#regions)) |
| A picker turned on only by its button | Also by its cursor in the Toolbar's well, the same `On`, the cursor chosen exactly while the picker is on | Decision 104: one place showing what a press on the photo does, for every owner |
| Turning a picker on gives its module focus (`color_picker_proxy.c:225-229`), and it turns off as the module loses focus (`imageop.c:2414-2416`) | No focused tool: turned on from the well, it leaves the Tools panel as it is; it turns off when another tool's control changes, another tool's Show on photo turns on (planned) or its own tool's section is collapsed | A cursor is a quick action; the area's label and the pointer's badge say whose picker it is |
| No label on the area | The active area or point labelled with its tool, and its picker where the tool has several | With the tool out of view, the photo still says whose picker it is |
| A pick only by the live picker | Also a one-shot pick (`Once`) of one step, for scripts, the MCP and assistants | One command for a client that names a region, rather than `On`, a move and `Off` |
| A click on the active picker's button turns it off (`color_picker_proxy.c:237-252`) | `On` for the active picker restarts it, its first reading applied; 79.6's app sends `Off` for that press | Every client turns a picker off by saying so; a script that sends `On` again gets a reading again |
| The Colour picker's button at the end of its statistic and colour mode row (`libs/colorpicker.c:772-775`) | In the section's header | The wireframe's layout |
| "Live samples", "display samples on image/vectorscope", "restrict scope to selection" | Stored samples, numbered; Show samples on the photo; Restrict the scopes to the selection | The wireframe's words; the behaviour darktable's, the vectorscope's marks included |

## Where the code lives

| Part | Where |
| --- | --- |
| `Picker`, `PickerKind`, `Space`, `Quad::Picker(id)`, `Row::Pickers`, `ProcessingTool::pick`, `Reading`, `Stats`, `Picked`, `Shown`, `Region`, `PickKeys`; `ProcessingDefinition::check`'s rules | `apertrust-tools`, its `definition` and a `pick` module |
| Each tool's pickers and `pick` | `apertrust-tools`, the tool's file |
| `Row::Patches`, `Patch::of_lch`; the tools' settings' statics | `apertrust-tools`' `definition` and `pick`; its `settings`, generated by its `build.rs` from the settings schema |
| The reduction kernel | `crates/engine/slang/picker.slang`, its WGSL generated beside it; `picker::Reducer` dispatches it |
| The active picker, stored samples, regions mapped through the geometry, keys, sample-only renders, applying a pick, the events | `apertrust-engine`'s `picker` module, its types, regions and reduction, and the session's `session/pickers.rs` |
| The staging pair | `crates/engine/src/readback.rs`, shared with the scopes and readouts |
| The on-photo overlay's primitives | `apertrust-ui`'s `overlay` ([gui.md](gui.md#on-photo-overlays)) |
| `PickerButton`, the picker's marks, the patches (`Patches`), the Colour picker's rows | `apertrust-ui`'s components, in the gallery |
| The quads and rows drawn | The app's `tools.rs` |
| The picker's mirror, its marks and commands on the photo, focus, the viewer bar's words, the live region, each tool's last patch and a pick's measured settings (`Picked`) | The app's `pickers.rs`, drawn from its Viewer |
| The tools' settings kept and their rows | The app's `settings.rs` (`ToolSettings`) and `tools.rs` |
| The Colour picker tool | The app's `colour_picker` module (planned) |
| A one-shot pick: `Region::pressed`; `PickerCommand::Once` | `apertrust-tools`' `pick`; the session's `session/pickers.rs` |
| The label on the area and the pointer's badge | `apertrust-ui`'s `overlay` (`Overlay::mark_label`, `icon` and `badge`) and the app's `pickers.rs` |
| The colour's name | `apertrust-color` |

## Tests

Built with 79.1:

- **The kernel** (`crates/engine/tests/picker.rs`): synthetic RGBA frames reduced on the GPU against `_color_picker_work_4ch` restated with the kernel's tree, the mean, the minimum and the maximum to the bit: boxes of one pixel at the corners and inside, a row, a column, odd sizes across workgroups' edges, the whole buffer, the buffer's edges, 64 boxes in one dispatch and one of them alone; darktable's box for a point and an area, a point on the right or bottom edge measuring nothing.
- **The session** (`session/tests/pickers.rs`): every command on the frame shown, each reading's region and the frame's own statistics under it; `On` taking each kind's last place after `Off`; a reading not sent again when nothing changed; coordinates clamped; a reading after the frame it measured, never waited for, and after its `Frame` through a session's thread; `On` refused with no photo and before live, for a tool's picker, and a move, switch or reset with no picker on; the Colour picker's picker on a view-only photo; off as another photo opens and as it closes, the readings on their way dropped, its place kept; an area through every orientation, measured where it lies, and a point set on a turned photo mapped back; a refused reduction turning the pickers off once, the frames going on.

Built with 79.3:

- **The kernel** (`crates/engine/tests/picker.rs`, and `picker.rs`'s own): a mosaic's boxes per Bayer and X-Trans colour, each colour's photosites counted from the mosaic's place on the sensor, against `_color_picker_work_1ch` restated with the kernel's tree, to the bit; denoise against `blur_2D_Bspline` restated, within 4 units in the last place; a mosaic and an RGBA buffer, denoised and not, on a device binding 256 KiB, read in row windows, equal to the same buffers bound whole; an operation's input and output in one reduction, each in its own place; workgroups dispatched across and down under a cap on workgroups across, the same bits as in one row; each colour's count against counting every photosite; a region's box in a viewport buffer and in the mosaic.
- **The session** (`session/tests/pickers.rs`, with `testing::Sample`'s three pickers, Colour balance RGB's and a picker on temperature of the test's own): `On` with no edit, a sample-only render publishing no frame, its first reading applied, then the change's values and frame, then the reading that frame measured at the operation, sent and not applied; each move applied; an upstream turn's reading never applied; a sample-only reading equal to a render's, bit for bit, at exposure's input and output, and from the edit cache after a move; a drag's steps unsaved and its release, with no move, saving once; turning off mid-drag saving; a switched tool turned on by `On`; the picker off as its tool changes, kept with `keep` and as another tool changes; the channel shown and the keys held as the pick reads them, a channel change applying nothing, a value past the hard limit held there; `On` refused for an unknown picker, an unknown setting and, for a tool's picker, on a view-only photo; the raw stage's mosaic per CFA colour against the synthetic mosaic, an empty fourth colour, a move rerunning rawprepare with no frame; a release while a reading to apply is on its way saving once, with that reading, and one while only an edit's reading is on its way saving at once, the next drag's first step a step either way; a picker on temperature in a tiled viewport with no highlight reconstruction reading nothing and applying nothing; two steps on their way at a release saving once, nothing left to save; a release whose area misses the image completing the drag's change, and one whose drag's step is still on its way waiting for it, the last reading sent the release's region; a release and the next drag's first step in one batch saved apart; a GPU error dropping a release that missed, the next render completing the drag; `Settings` applying the latest reading measured, not the last applied, and sending its reading, and a refused `Settings` pick turning the picker off with the last reading kept, through a test tool with a setting, and `Settings` mid-drag saving the drag's step first; a waiting reading replaced passing its application on; a GPU error rendering again without the reduction only where one was in the render.
- **Measured** (ignored, with the corpus): the moved area's budgets of [Cost](#cost).

Built with 79.5 (`crates/ui/tests/picker.rs`): the button's click, right-click, Ctrl-click, Space, Enter, Shift+F10 and the Menu key, each reported, any of them while on `Off`; Ctrl alone picking an area, the Mac's Cmd and Control too, Ctrl+Shift a click; on a quad, a right- or middle-click a click, Ctrl-click (with any button) and Shift+F10 an area, its tooltip's quad hint; its AccessKit node toggled, its description the tooltip, in each language and with Cmd on the Mac; disabled, no press reported and the reason its description; the gallery's Pickers section photographed at twice the size, dark and light (`picker-dark`, `picker-light`).

Built with 79.6 (the app's `pickers.rs`, `tools.rs`, `mirror.rs` and `main.rs`, and `apertrust-ui`'s `picker.rs` and `shell.rs` tests): `testing::Sample`'s three pickers in a scene wired as the app wires them, the quad's and the row's buttons on, off and pressed, a right-click and Ctrl-click with the keys held, the focus moving to the photo; a point's drag and an area's by its corner, strictly within 6 points, or drawn anew, Ctrl-click and a right-click on the photo, the Mac's Control-click a right-click; every key of the table, Esc giving the focus back; screen readers' Increment and Move down; the live region waiting for the rest and the reading, and forgetting as the picker changes or turns off; the focus given back lasting two frames; the mirror's part through refusals, a picker turned off before its first reading, another photo and a close; a pick waited for until its `Saved`, a release answered by `Saved` alone leaving a slider's drag after it unaffected; in the engine, a pick that writes nothing answered with a `Saved` through it, and the end's `through` after `Off`, a change of its tool, a failure, another photo and a close; the bar's words; with the picker's node focused, the arrows, Space, Backspace and Delete reaching it and not Next Photo, Previous Photo or an action bound to them; the panel's pickers disabled saying why, and none drawn for a tool whose pick is not built; a slider's picker quad and the viewer bar's words in the design system; Sample's rows of pickers, its quad on, beside the marks, photographed at twice the size, `pickers-dark` and `pickers-light`.

Built with 79.7: Exposure's pick against darktable's arithmetic restated in f64 on readings made for the test (`crates/tools/src/exposure.rs`): correction on an image that is not a raw and on colour raws with biases 0, −0.67 and +1.3 EV, compensated and not, at target lightnesses 18, 50, 72.5 and 100 %, within 1e-4 EV; an area of colour made grey of its L first; the patch's L and sRGB; nothing written where the exposure is already the one picked; a black at or above the new white put 0.01 below it, over a colour raw's own black too; a target of 0 not finite and an area of black past +18 EV; the input profile colorin's and the black the exposure operation's. Middle grey picking exactly what Lightness at 50 picks, whatever Lightness holds (E52); a pick toward the reference exactly Lightness's at its L, and none with no reference set; the reference picker handing back measure mode's L within 1e-3 and writing nothing (E54); `picker_ready` false toward Reference with none set. E50 and E53 (`crates/tools/tests/exposure.rs`) render each pick; the session refuses `On` with `NotReady`, turns the picker off as `Settings` clears the reference, and makes no step for the reference picker's pick (`exposure_s_picker_waits_for_a_reference…`, `the_reference_picker_hands_its_lightness_back…`); Last pick before, after and the change, `Short` at ±18 EV with the L reached and the black moved, `clipped` at 0.99 × one channel's `input_maximum` alone and not at 0.98 (`crates/tools/src/exposure.rs`), `input_maximum` in the session's reading (`a_reading_carries_its_operation_s_input_maximum`), and the app's rows, the live region's words and the snapshots `exposure-last-pick-*` ([tools/exposure.md](tools/exposure.md#results)). The tools' settings declared as their rows (`crates/tools/src/settings.rs`); `Settings` re-applied with no settings handed back (the session's `settings_apply_the_latest_reading…`); a value that is not finite, Exposure's at a 0 % target, left out with the patch still shown, and the same reading at 50 % writing Exposure (`a_value_a_pick_gives_that_is_not_finite…`); the patches to AccessKit and in the gallery (`crates/ui/tests/picker.rs`, `picker-dark`, `picker-light`); Picker target and Lightness live in the app, each option's tooltip, every option to be chosen, Lightness shown while chosen, in percent, on darktable's ramp of greys, the quad's tooltip and the viewer bar by the target, measured settings taken once, the input patch gone at undo, redo, a history move and a reset, and photographed at Middle grey and at Lightness (`exposure-middle-grey-dark`, `exposure-lightness-dark`; `crates/app/src/tools.rs`, `pickers.rs`), and `settings.toml`'s `[tools.exposure]` read and written byte for byte, an old `area_mode` key left alone.

Built with 79.9: `chroma_adapt_pixel`, `xyz_to_rec709_d50` and `illuminant_xy_to_rgb` against darktable's formulas restated in f64 (`apertrust-color`), and the adaptation against the `channelmixerrgb` kernel's (`c8_chroma_adapt_pixel_is_the_kernel_s_adaptation`); Colour calibration's readout, CCT and swatch, its correction pick against `_auto_set_illuminant` restated in f64 within 1e-5 in `x` and `y` over every adaptation, three mixers and channel mixing on and off, its measure pick, its patch and its "(active)" title (`crates/tools/src/colour_calibration.rs`); C8 (`crates/tools/tests/colour_calibration.rs`, [tools/colour-calibration.md](tools/colour-calibration.md#results)); the area colour mapping's settings declared as their rows (`crates/tools/src/settings.rs`); the session's `On` writing the light in one step, `Settings` applying the latest reading again in correction mode and handing nothing back in measure mode, and the readout following `SetControl` (`session/tests/pickers.rs`); the app's readout row, hidden with the adaptation none, the mapping live, its three-line readout and its title, `settings.toml`'s `[tools.channelmixerrgb]` byte for byte, and the CAT tab after a pick photographed (`colour-calibration-mapping-dark`, `-light`).

Built with 79.10: darktable's `XYZ_to_Ych` against its formula restated in f64 (`apertrust-color`); Colour balance RGB's pick against `color_picker_apply` and `pipe_RGB_to_Ych` restated in f64 (`crates/tools/src/colour_balance_rgb.rs`): each hue picker's opposite hue and, with `command`, its own, on red, green, blue, a skin tone and a cyan, within 4.0e-5° of the bound of 1e-4°, from −150° to 390° before the session wraps it, the chroma within 8.6e-7 relative, a neutral area's chroma 0 and darktable's hue for none, 30° and 210°, White fulcrum within 3.5e-7 relative from a maximum that is not the mean and Contrast grey fulcrum within 4.6e-8, each picker writing only its `writes`, an unknown picker nothing; B8 (`crates/tools/tests/colour_balance_rgb.rs`, [tools/colour-balance-rgb.md](tools/colour-balance-rgb.md#results)); the session's `On` turning the tool on and writing its own hue with `command`, darktable's below 0° wrapped onto 0° to 360°, then the opposite, and White fulcrum of an area black at the operation's input writing nothing and making no step (`session/tests/pickers.rs`); the app's six quads and a press sending `On` with the keys held (`colour_balance_rgb_draws_its_six_pickers…`), and the 4 ways tab with its quads photographed (`colour-balance-rgb-pickers-dark`, `-light`).

Planned, with each part:

- **The kernel:** each space against darktable's conversions restated, the hue's fourth channel included.
- **Regions:** the default point and area; corners mapped back and forward through orientation's `flip` (the first geometry) as darktable's sort-and-average gives them, then through each geometry as it lands; an area outside the image measuring nothing; a point the pixel it falls in.
- **The session:** a drag one step of source Auto, merged with a quick repeat (history's 9.1); the picker off after Undo, Redo and a `GoTo` that change its tool; each tool's own bounds and a hue wrapped, with its pick; a non-finite pick dropped with no step; the first moved area after a rebuild capturing again; a refused reduction at the operation rendering the frame again without it; stored samples added, locked, loaded, stored into and removed.
- **Each tool's pick:** pure tests from readings to writes, each against darktable's `color_picker_apply` restated, the reading darktable's own on a fixed area where one was recorded: W-rows for White balance, and each tool's page's rows.
- **By hand against darktable:** each tool's pick on the corpus' 7R V and X-T5 at the same areas in darktable 5.6.2's GUI, the values written recorded in the tool's page's Results, with no bound, as the scale differs ([The scale](#the-scale)).
- **The app:** each tool's pickers drawn as its pick lands; stored samples' marks and `Load` on the photo; the Colour picker's rows, values in every colour mode, and their snapshots. Built with decision 104's 104.7: the area's label in snapshots over light, dark and busy photos, at each edge and the corner's fallback, focused, and at interface sizes 100 % and 200 %, its placement and words unit-tested; the badge and its flip. Built with 104.5 and 104.6: Esc from any focus but a typed field or an open list; A held panning over a picker and giving it back; the right-click's reset; the announcements with their reasons; a picker cursor's face, list item and key sending its picker's `On`, and its face again `Off` ([cursors.md](cursors.md#tests)).
- **The gestures** (decision 105, with each step of [The plan](#the-plan)): a press moved 6 points then released a click, its area or point as the press sent it, and one moved 7 points a drag, whatever the time held, a press held 1 s still a click; a corner and a point's mark taken and let go within the threshold sending nothing; a long press on a touch screen resetting nothing; an area picker's Ctrl-click and Space sending no `Switch`, the Ctrl-click a click with `command`; the viewer bar's words by kind; the click's area at Fit equal to `Region::pressed`'s and at 200 % and 800 % the same size in points; on a picker with both kinds a click a `Point`, a drag an `Area`, a press on the point's mark moving it, no `Switch` from the photo; the session refusing `Switch`, a `Point` for an area picker and an `Area` for a point picker with `OneKind`, the picker's kind and place unchanged, and `Switch` on a picker with both kinds as before.

Built with 79.17, the one-shot pick:

- **The tools** (`pick.rs`): `Region::pressed` against darktable's new box restated (`darkroom.c:4064-4073`) at landscape, portrait, square and wide sizes, clamped at the edges; the app's press on the photo makes its new area with it, its test unchanged.
- **The session** (`session/tests/pickers.rs`, and `history.rs` for the default steps): with `testing::Sample`, a click with an area picker reading the area a press there starts and writing what the tool's `pick` gives for that reading, one step of source Auto answered by a `Saved` through it; the events in order, `Picker` with its reading and no frame part, `Values`, `History`, `Frame`, then `Picker` with none; the picker off and its own area still the default for its next `On`; a point read as a point by a picker that takes points, an area as sent in any corner order, a point outside the photo clamped, a picker on turned off first; a pick that writes nothing making no step, writing no file and answered held; two quick one-shots two steps, one Undo each; Colour balance RGB, off, turned on by its White fulcrum picker on an area black at its input, whose pick writes nothing, in one step holding the turning on alone, held until then, Undo turning it off, and a switched test tool, off, whose pick writes, one step holding both; ended before its reading by `Off`, another photo, a close, a second one-shot or a command on the edit naming its tool, the turning on a step of its own, the command's step after it, and by a command on another tool only so far, the turning on saved as a step of its own and the pick a step of its own once its reading arrives; an Undo before its reading, of a tool it turned on or one already on, ending it, its reading writing nothing and Redo putting back what the Undo took, and Compress, Truncate and Discard History ending it too; the pickers failing with its turning on held sending the step it made; a move, switch or reset while only a one-shot is on refused (`NoPicker`); with the end among the default steps, the turning on and Exposure's pick one step, not two; a point on a photo turned clockwise reading the part of the scene it read upright, and what `On` and a move there read, bit for bit; an area picker's click on the turned photo's shape; refused, writing nothing and making no step, before the photo is live, for an unknown picker, a tool the session lacks, the Colour picker's picker, an unknown setting, while a tool is hidden, a picker not ready and on a photo open for viewing only; White balance's on the synthetic raw writing the neutral coefficients of W15's rule and from image area, and Exposure's, one step each.

## The plan

Phase 2 ([roadmap.md](roadmap.md#pickers)), one pull request each, each reviewed before the next that needs it. 79.1, 79.2 and 79.4 can start at once, 79.4 before any mark is drawn on the photo; switched tools (decision 32) come before the pickers of tools that are switched; a pick's own step, of source Auto, follows history's 9.1, and until then a pick joins the next step, as `Set` does.

| # | Delivers | Touches | Out of scope |
| --- | --- | --- | --- |
| 79.1 | Built: the reduction kernel, `picker_rgba` and `picker_combine`, on the frame; `Region`, `Stats` and `PickKeys` in `apertrust-tools`; `PickerCommand` for `PickerRef::ColourPicker` alone, with `On`, `Point`, `Area`, `Switch`, `Reset` and `Off`; regions mapped through orientation; `Event::Picker` with its frame part; the staging pair shared as `readback.rs`; the frame samples' cost measured at 2400×1600 on the M4 | tools, engine | tool pickers, stored samples, the app |
| 79.2 | Built: `apertrust-tools`'s side: `Picker`, `ProcessingDefinition::pickers`, `Quad::Picker(id)`, `Row::Pickers`, `ProcessingTool::pick` with its `channel` and `last`, `Picked` and `Shown`, `check`'s rules, `testing::Sample`'s pickers; the built definitions' quads naming their pickers | tools | the engine, every real tool's pick |
| 79.3 | Built: the engine's side: the reduction at an operation's input and output, `picker_mosaic`, denoise, row windows; `On`'s capture and the operation as the edited instance; sample-only renders through `Pipeline::sample_writes`, as frames through `Pipeline::writes`; applying a pick as a gesture, the tool's last reading; turning off; `Settings` and `Channel`; the moved area's budgets measured on the M4 | engine | other spaces, the app |
| 79.4 | Built: the design system's on-photo overlay primitives ([gui.md](gui.md#on-photo-overlays)): the line in its three weights, the square and circle handles, the sampled-colour centre and the focus ring on the photo, in points over a photo's rectangle, in the gallery with snapshots over light, dark and busy photos | ui | every mark on the photo |
| 79.5 | Built: `PickerButton` with its states, keys and AccessKit, and the picker's marks, `PickerMarks`, drawn with 79.4's primitives, in the gallery with snapshots | ui | the app |
| 79.6 | Built: the app: quads and `Row::Pickers` drawn with their state, the marks over the photo, the viewer bar's words and their messages, the pointer's and the keys' tables, the keys taken before the app's actions, focus, the live region; driven by `testing::Sample` | app | real tools |
| 79.7 | Built: Exposure's picker: its pick, its settings in the settings schema, Picker target with Middle grey and Lightness (decision 88); E50 and E52 | tools, app, settings schema | Auto |
| 79.8 | Built: White balance's picker: `spot` turned on by Settings' from image area (`PresetRow::pickers`), its pick, the label "user modified" when history is read again (`ProcessingTool::redrawn`); W15 and its rows on the tool's page | tools, engine, app | the button bar |
| 79.9 | Built: Colour calibration's picker: the readout row with CCT and the swatch (`Row::Pickers::readout`, `ToolState::readouts`), Area colour mapping's settings, its title "(active)", its pick in both modes; C8 | tools, engine, app, settings schema | the colour checker |
| 79.10 | Built: Colour balance RGB's six pickers, `command` for the colour itself; B8 | tools, color, engine's tests, app | the mask display |
| 79.11 | AgX's five pickers, Dynamic range scaling as their margin, Auto tune levels as a `Row::Pickers` | tools | Read exposure, Show curve |
| 79.12 | RGB curve's two pickers and its graph's readings, with RGB curve (Phase 3) | tools, app | blending |
| 79.13 | RGB levels' three pickers, with RGB levels (Phase 3) | tools, app | Auto region |
| 79.14 | `SamplesCommand` and `Event::Samples`: stored samples measured on the frame, locked, loaded, stored into, removed and cleared | engine | the app |
| 79.15 | Restrict the scopes to the selection: `Command::Scopes` gaining the Colour picker's point or area, the scopes' kernel counting within it | engine | the app |
| 79.16 | The Colour picker, a utility tool, hidden by default, its place to be decided: the header's picker button, with its cursor group in the well (decision 104's 104.8), the statistic and colour mode, the live sample with its 52-point patch and its Point \| Area choice (decision 105's 105.5), Add sample, numbered stored samples, Show samples on the photo with the vectorscope's and RGB curve's marks, Restrict the scopes, its settings and actions | ui, app, settings schema | blending |
| 79.17 | Built: the one-shot pick ([A one-shot pick](#a-one-shot-pick)): `Region::pressed` in the tools; `PickerCommand::Once` in the session, with its tests, for scripts, the MCP and assistants; decision 104's 104.1 | tools, engine | the app, the cursors |
| 105.1 | The threshold (decision 105): a press a click while within 6 points, by distance alone, for every picker's press, corner and point; egui's long press not a right-click | app | the kinds' rules, the click's area |
| 105.2 | A picker that takes one kind keeps it on the photo: no `Switch` from its Ctrl-click or Space, the Ctrl-click a click with `command`; the viewer bar's area and point words, `picker-bar-point-what` gone | app, ui's locales | the session, pickers with both kinds |
| 105.3 | The session keeps a picker that takes one kind to it: `Switch`, a `Point` for an area picker and an `Area` for a point picker refused, `SessionError::OneKind` | engine | the app |
| 105.4 | The click's area a fixed size on screen: its size at Fit at every zoom, worked out from the view; `Once` and `Region::pressed` unchanged | tools, app | the threshold |
| 105.5 | A picker with both kinds: a click a point, a drag an area, a press on the point's mark moving it, no switch on the photo; its viewer bar words; with the Colour picker tool (79.16), its Point and Area choice, and RGB curve's show colour following with 79.12 | app, ui's locales | the section's other rows |

105.1 comes first; 105.2 and 105.4 follow it, 105.3 after 105.2, so the app never sends what the session refuses; 105.5 with 79.16.

**Later, with their tools:** Filmic RGB's pickers and a quad on a choice row (Phase 3); the colour equalizer's, with `Space::Jzczhz`, after its tool (Phase 3); Colour calibration's colour checker, many boxes in one reading (Phase 3); masks' overlays on 79.4's primitives and blending's pickers, with `Space::Lch` and `Space::Hsl` and a layer's instance in `PickerRef` (Phase 4); picking by coordinates through MCP (Phase 6).

## Open questions

- **A click on an area picker moving the area,** keeping its size, in place of a new click's area round the press: it would keep a drawn area through a stray click and answer a click's size at every zoom too, but departs from darktable's press. Tried in the UX pass after Phase 3 ([roadmap.md](roadmap.md#the-ux-pass)). Meanwhile a stray click replaces the area, which is drawn again by hand (decision 105).
- **The Colour picker's Point | Area choice and the well's Start as:** one setting or two, settled with the Colour picker tool (79.16).
