# Colour pickers

A colour picker reads the colour of a point or an area of the photo. A module's picker turns what it reads into the module's controls: Exposure's sets the exposure that maps the area to a target lightness, White balance's the coefficients that make it neutral. The Colour picker module shows what the photo holds under a point or an area, the live sample, and keeps numbered stored samples to compare. This document describes them as one API: the session's commands and events, how the engine measures on the GPU, how a module declares its pickers and what it does with a reading, the picker button and the area on the photo, the Colour picker module, and how the work is built. The session's other commands and events are in [session-protocol.md](session-protocol.md) and [events.md](events.md); history's steps in [history.md](history.md).

**Status.** [Decision 79](decisions/0079-colour-pickers-one-api-for-modules-and-the-colour-picker.md) records the design, with the options considered. Nothing of it is built: the modules' definitions hold their picker quads (`Quad::Picker`), which the panel does not draw, and the rows darktable gives the pickers' settings are drawn disabled (`Row::ToolSetting`, `Row::Action`). It is planned in Phase 2, in the pull requests of [The plan](#the-plan).

## What it serves

| Client | What it needs |
| --- | --- |
| A module's pickers | Exposure, White balance, Colour calibration, Colour balance RGB, AgX, RGB curve and RGB levels in the first pass, then Filmic RGB and the Colour equalizer; each reads its operation's input, or its output too, and writes its controls as one step |
| The Colour picker module | darktable's color picker utility: the colour under a point or an area of the photo as it is shown, the live sample and stored samples, statistics and colour modes, samples shown on the photo, the scopes restricted to the area |
| Pickers that only show | RGB curve's and blending's "show colour", the Colour equalizer's hue: drawn on the module's graph, writing nothing |
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
        A["The active picker<br/>a module's, or the Colour picker's"]
        L["Stored samples<br/>each locked or not"]
        R["Each picker's last point and area"]
    end
    A -->|"module picker"| M["Reduction at its operation's<br/>input and output, in the viewport pipe"]
    A --> F["Reduction on the frame"]
    L --> F
    M --> P["Module::pick → one change, source Auto"]
    M --> E1["Event::Picker"]
    F --> E1
    F --> E2["Event::Samples"]
```

- **One active picker** per session, as darktable has it: a module's, named by its module and its id, or the Colour picker module's (`PickerRef::ColourPicker`, darktable's primary picker with no module).
- **The live sample** is what the active picker measures on the frame, darktable's primary sample; **stored samples** are darktable's "live samples": areas or points copied from the active picker, numbered, each measured on every frame unless locked, any number, coexisting with the active picker. The interface uses the wireframe's words, live and stored; darktable's code names the second kind live samples.
- **Both are view state:** the session keeps them, as it keeps what the scopes count and the eye's hidden modules; nothing is saved in the edit, and they stay across photos for the session's life, as darktable keeps its "live samples" across images. The active picker turns off as another photo opens.
- **A module picker measures twice:** at its operation's place, for the module; and on the frame, for the Colour picker module, which shows the colour under the active picker whichever picker it is, as darktable's utility shows its primary sample.

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

## Measuring on the GPU

### Where

**A module's picker measures at its operation's place in the live viewport pipe:** the operation's input, and with `output` its output, in the buffers the viewport's plan already holds, never a host copy of the image. Only a small result comes back: three statistics of four channels, 48 bytes a region.

- **The operation's input or output buffer** is what the reduction reads, right after the operation's last pass in the render's encoder, while both are intact: the planner writes an operation's output into a buffer that is not its input (engine.md, Planning), and nothing writes either before the next operation runs.
- **Its colour space** is the stage's: the mosaic in the raw stage, camera RGB, the working space, as each operation's slot names it (decision 21). A picker's `space` converts each pixel before its statistics, as `picker_cst` does ([The kernel](#the-kernel)).
- **The frame samples,** the Colour picker module's, read the display buffer the frame was copied into, after colorout and before any rounding to 8 bits, as the scopes count it (decision 19): darktable's gamma input in its preview pipe (`pixelpipe_hb.c:3017-3027`). In a zoomed view they read the display output of the small whole-image pipe the scopes read then ([gui.md](gui.md#the-scopes)), never the visible region alone, so their values are the same at every zoom. Denoised where the sample is: the active picker's with its picker's denoise, a stored sample with the denoise it was copied with (`libs/colorpicker.c:613`, `pixelpipe_hb.c:1154`). Today the frame is sRGB, which is darktable's default display and histogram profile, so its values are both darktable's `display` and `scope` RGB.

### The scale

**The fitted viewport's,** darktable's preview pipe's place in Apertrust (decision 63: the export's processing at the view's scale, never darktable's preview shortcuts):

- **After the downscale** (every operation after demosaic: Exposure, Colour calibration, the tone mapper, Colour balance RGB, the curves): at the viewport's size, 2400×1600 for a full-screen photo, which is darktable's preview pipe's role.
- **In the raw stage** (White balance over `temperature`): at full resolution, since the raw stage runs there in every pipe (decision 12). White balance's mean is then the export's photosites' mean over the area, where darktable's is its preview's downscaled mosaic's; the two agree but for the area's edges, and a minimum or a maximum, which White balance does not read, would differ more.
- **A zoomed view** (Phase 2's 100% view, which renders only the visible region) still measures the fitted whole image: the small whole-image pipe the zoomed view keeps for the scopes and readouts (engine.md, Readouts) holds every buffer a picker reads and the frame its samples read, so an area outside the visible part measures, as darktable's preview pipe does, and a pick gives the same values at every zoom. That pipe is still open in decision 19 and comes with the zoomed view; pickers in a zoomed view depend on it, and until it is built the view is fitted, its frame the whole image.
- **Exports, stills, tiles, golden cases and derivative passes** never measure: pickers are the live viewport's, as readouts are.

### The kernel

The engine's own kernel, `crates/engine/slang/picker.slang`, one for every operation, as the scopes' kernel is for every frame (in `pipeline::KERNELS` for the naga test). It is no operation's pass: no derivatives, no golden case, no fusion.

| Entry point | Reads | Writes |
| --- | --- | --- |
| `picker_rgba` | An RGBA `f32` buffer and up to 64 boxes in its pixels, with each box's space and denoise flag | Per box and per workgroup: the sum, minimum and maximum of each of four channels |
| `picker_mosaic` | The mosaic, its CFA pattern (Bayer `filters` or the X-Trans 6×6) and the boxes | The same per CFA colour, with each colour's count |
| `picker_combine` | The partials | Per box: the mean, minimum and maximum, in a fixed order |

- **Statistics** are darktable's `_update_stats_by_ch` (`color_picker.c:35-44`): the mean is the sum over the box's pixels divided by their number, or on a mosaic each CFA colour's over its own count (`:299-304`); the minimum and the maximum are exact.
- **The sums** are pairwise, a tree in workgroup memory, then the workgroups' partials added in their index order by one workgroup, so the result never depends on scheduling. On the CPU the same tree gives the same bits; across GPUs `f32` addition is correctly rounded, and a backend that contracts or reorders adds may differ by a few units in the last place, which no pick's write can feel.
- **Denoise** (`DT_COLOR_PICKER_DENOISE`) is darktable's `blur_2D_Bspline` at one scale (`color_picker.c:480-502`, `src/common/bspline.h`): the 5×5 B-spline, weights 1, 4, 6, 4, 1 over 16 on each axis, its edges as darktable's, negatives kept. darktable blurs the whole image first; the kernel blurs only the box's pixels, reading two pixels round it, which gives the same values.
- **Spaces,** darktable's `dt_color_picker_helper` (`color_picker.c:505-543`): `Space::Operation`, the stage's own values; `Space::Lch` for a Lab stage (`dt_Lab_2_LCH`); `Space::Hsl` for an RGB one (`dt_RGB_2_HSL`); `Space::Jzczhz` for an RGB one through the working profile's matrix to XYZ D50, D65, JzAzBz and JzCzhz (`:59-82`). In each converted space the fourth channel is the third moved half a turn, `[2] < 0.5 ? [2] + 0.5 : [2] − 0.5`, as `_update_stats_4ch` makes it (`:53`): the hue in LCh and JzCzhz, but L in HSL, whose third channel is lightness. In `Space::Operation` the fourth channel is the stage's own fourth, and on a mosaic the fourth CFA colour's. A mosaic is read per CFA colour only (`:547-556`). The spaces other than `Operation` come with their first picker ([The plan](#the-plan)).
- **Rules:** `fp32`; no write at a run-time index, the four channels through `set_at` and `add_at` (decision 10); no NaN checks, the operations before it sanitising their outputs; the `slang-kernels` skill's "A count" adapted to sums, minima and maxima.

**Several boxes in one dispatch:** the module picker's one box at its operation, and on the frame the active picker's and every unlocked stored sample's, so the frame is read once however many samples there are. Colour calibration's colour checker will read its patches the same way, up to its chart's patches in one dispatch.

### When it runs

A reading has a key, as a readout's does (decision 19): what it reads, the box in that buffer's pixels, its space and denoise. A reading runs only while its key differs from the last one measured.

- **A render** that rewrites the picked operation's input or output (a change at or before the operation) dispatches the module picker's reduction in the same encoder after the operation's last pass. A render that restarts after the operation, as a drag of a later module does, leaves its key unchanged and measures nothing at the operation.
- **A region change with no edit** needs no frame, since the frame is unchanged: the session runs a sample-only render, from the newest cache point at or before the operation through the operation, then the reduction, and publishes no frame, as a readout asked without a current reading does (decision 19).
  - **Turning a module's picker on makes its operation the edited instance** (engine.md, Caching), as darktable focuses the module (`color_picker_proxy.c:228`). `On` runs a capture: its sample-only render copies the operation's input into the edit cache, as the first render after a change does. After the downscale an input reading of a moved area then reads the edit cache directly, with no pass of the pipeline, and an output reading runs the operation's own passes from it.
  - **The override lasts** until another operation's key changes, which makes that operation the edited instance as any change does. After a rebuild, which captures nothing (engine.md, A rebuild keeps what still holds), the first moved area renders from the newest cache point at or before the operation, and captures again.
  - **Sample-only renders submit as frames do,** through `Pipeline::writes` and the submit gate, one turn each, behind the live frame and before background stills.
  - **In the raw stage** there is no edit cache: White balance's moved area reruns rawprepare from the uploaded mosaic, then measures temperature's input.
- **The frame samples** are measured after each frame while the active picker or an unlocked stored sample is there, in the scopes' submission after the frame's copy (decision 19), and on a region or sample change on the frame shown, with no render.
- **A locked stored sample** is not measured again; the session keeps its last reading.

**Readback** is the scopes' and the readouts': the results copied into whichever of the pickers' two staging buffers is free, mapped and polled between commands, never waited for, the latest replacing the last while both are busy, so the last reading always arrives (`crates/engine/src/readback.rs`, the helper the scopes, readouts and pickers share). A frame never waits for a reading.

### Memory, strips and failures

- **Buffers:** the boxes' uniforms, the partials and the results, the session's own, counted in the plan's memory as a readout's counts buffer is. The partials are sized for the frame's 64 boxes, the most one dispatch takes, about 2.9 MB at 2400×1600, which also holds one box over a 61 MP mosaic at 64 × 64 photosites a workgroup; more boxes take more dispatches.
- **Strips:** a buffer larger than one binding, as the 61 MP mosaic on lavapipe, is bound in row windows, each window's workgroups writing their partials at their index in the whole box, so the result is the whole box's (decision 14). A tiled viewport's raw stage is whole in strips, and every buffer after the downscale is at the viewport's size, so a picker never reads a tile.
- **Failures:** a reduction the GPU refuses, inside the frame's submission, makes the session render that frame again without pickers, then turns pickers off for the session, reported once as `SessionError::Pickers`; a mapping that fails does the same. A picker never costs the frame, as the scopes and readouts never do.

### Cost

Estimated until 79.1 and 79.3 measure them on the M4:

- **Frame samples:** a box over the whole 2400×1600 frame reads 61 MB, about the scopes' totals pass, 0.64 ms of GPU time; a point or a small area, under 0.05 ms.
- **Exposure's moved area:** the reduction on the edit cache, under 1 ms, the reading a frame after the move at most.
- **White balance's moved area at 61 MP:** rawprepare again, 4.6 ms, and the default area's 56 MP of photosites read, about 2.5 ms. The pick it applies then changes the coefficients, and that change reruns the raw stage and demosaic at full resolution, about 240 ms at 61 MP on the M4, as any White balance change does (engine.md, Where the viewport's scale applies).
- **Budgets,** measured in 79.3 on the M4 with the 7R V at 2400×1600: a moved area's reading within 16 ms of its command after the downscale and within 50 ms in the raw stage. A raw-stage miss keeps the picked operation's input as a cache point while its picker is on: 244 MB at 61 MP, an eighth of `engine.memory_budget`'s 2 GB default, counted against it, the first thing freed when an allocation fails.

## Through the session

### Commands

`Command::Picker(PickerCommand)` and `Command::Samples(SamplesCommand)`. Neither is a command on the edit (`on_edit`): they are view state, as `Scopes` and `Readouts` are, so they end no gesture. `On` is refused before the photo is live and, for a module's picker, on a photo open for viewing only (below); the Colour picker's picker and `Samples` work on a photo open for viewing only. A pick's own write is a change on the edit ([What a pick writes](#what-a-pick-writes)).

```rust
pub enum PickerRef {
    /// A module's picker, by its module's id and the picker's id in its definition.
    Module { module: &'static str, picker: &'static str },
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
    On { picker: PickerRef, area: bool, keys: PickKeys, tools: Vec<(&'static str, Value)>, channel: Option<&'static str> },
    Point { at: [f32; 2], done: bool, keys: PickKeys },
    Area { from: [f32; 2], to: [f32; 2], done: bool, keys: PickKeys },
    Switch,
    Reset,
    Tools { module: &'static str, tools: Vec<(&'static str, Value)> },
    Channel { channel: Option<&'static str> },
    Off,
}
```

| Command | What it does |
| --- | --- |
| `On { picker, area, keys, tools, channel }` | Turns the picker on, the one on before it off. A picker with both kinds takes an area with `area`, else a point; one kind, that kind. It starts at the picker's last point or area this session, else the default. Its module is turned on, as darktable's is, and its operation becomes the edited instance. `tools` are the module's picker settings ([Tool settings](#tool-settings)); `channel` is the channel tab the module shows, RGB levels' and RGB curve's. Its first reading is applied, one completed change |
| `Point { at, done, keys }` | The active picker's point moved, a click or a drag's step; `done` on the release |
| `Area { from, to, done, keys }` | The active picker's area from one corner to the other, a drag's step or its release; any order of corners |
| `Switch` | darktable's Ctrl-click on the photo (`darkroom.c:4017-4034`): point to area or back, each at its last place; a completed change |
| `Reset` | The picker's default for its kind: the default area, darktable's right-click on the photo with an area (`darkroom.c:4125-4132`), or the default point, which Delete alone reaches, darktable's right-click doing nothing with a point; a completed change |
| `Tools { module, tools }` | The module's picker settings changed: kept, and the module's last reading applied again where its pick reads them, as Exposure's target lightness in correction mode applies again (`exposure.c:1154-1176`). The session keeps each module's last reading after its picker turns off, for the photo, as darktable keeps `picked_color`; with none, nothing is applied |
| `Channel { channel }` | The channel tab the active picker's module shows changed, as the client sends `tools`; kept for its next pick, applying nothing, as darktable's tab switch applies nothing (`rgblevels.c:756-766`) |
| `Off` | The active picker off; its point and area kept for its next `On` |

- **Refused,** each as `SessionError::Refused` with the command's number: `On` before the photo is live (`NotLive`) or, for a module's picker, on a photo open for viewing only (`ViewOnly`), as commands on the edit are (decision 77); a picker no module declares, or whose operation the plan lacks after its module is turned on (`UnknownPicker`); a point or area command, `Switch` or `Reset` with no picker on (`NoPicker`); `Tools` naming settings the module's pickers do not read (`UnknownTool`).
- **The region** is in the photo's coordinates, clamped to 0 to 1. A drag sends its steps with `done: false` and its release with `done: true`; the session makes the steps one gesture.
- **Keys** ride with every command that can apply a reading, so a module reads them as darktable reads the modifiers when its reading arrives: Colour balance RGB's hue picker takes the colour itself with `command`, else its opposite (`colorbalancergb.c:1289-1290`); RGB curve's create-curve picker makes a positive curve with `command`, a negative one with `shift` (`rgbcurve.c:557-564`).

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
| `Load { id }` | darktable's first right-click on a sample's patch (`:539-582`): the sample's place into the Colour picker's own picker, turning it on whenever it is not the active one, a module's picker on then being turned off; the sample marked loaded. On the photo, a right-click on a sample while samples are shown loads its place into the active picker instead, whichever it is, only where its kind is one that picker takes (`darkroom.c:4087-4123`) |
| `Store { id }` | darktable's second right-click while a sample is loaded (`:543-558`, `:573-582`): the Colour picker's place into the sample, then the Colour picker's picker turned off |
| `Remove { id }` | The sample gone (`:508-521`) |
| `Clear` | Every sample gone and the Colour picker's picker off, its reset (`gui_reset`, `:887-929`) |

**Ids and numbers.** `id` is a sample's stable handle in commands and events, counting from 1 in the session and never reused. The number shown beside a sample, and in its accessibility names, is its place in the list, from 1, as darktable lists them, so removing one renumbers those after it.

### Events

Two facts ([decision 59](decisions/0059-one-event-system.md)), each its whole topic:

```rust
/// Fact::Picker. `None` while no picker is on.
pub struct PickerReading {
    /// The commands the frame or the render it measured includes, as `Frame::through` counts them.
    pub through: u64,
    pub picker: PickerRef,
    /// As drawn, in the photo's coordinates.
    pub region: Region,
    /// A module picker's statistics at its operation; `None` for the Colour picker's.
    pub module: Option<ModuleReading>,
    /// The frame's colour under the region; `None` until measured, or where the region misses the frame.
    pub frame: Option<FrameReading>,
}

pub struct ModuleReading {
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

- **`Event::Picker(Option<PickerReading>)`** is sent when a picker turns on, once its first reading arrives; after each reading that changed its module part or its frame part; and `None` as it turns off. During a drag of another module the frame part changes every frame, so a reading follows each frame while a picker is on, about 200 bytes.
- **`Event::Samples(Vec<StoredSample>)`** is sent after each command that changes the list and after each frame that changed an unlocked sample's reading.
- **Order:** a reading after the `Frame` it measured, as the scopes' readings come, and before the `Values` of the change a pick applies. Readings of a photo that closed never arrive.
- **The types** live in `apertrust_engine::picker` and `apertrust_modules::pick`, re-exported by the session: plain data that moves into `apertrust-protocol` unchanged. `Region`, `PickKeys`, `Stats` and `Shown` are the modules crate's, since `Module::pick` takes and gives them.

### Turning off

The session turns the active picker off, sending `Picker(None)`, when:

- `Off`, or `On` for another picker;
- another photo opens, or the photo closes (`darkroom.c:1100-1102`);
- its module changes by any command other than its own picks: a control set, reset or kept, a preset, `ResetModule`, `Reset`, Undo, Redo or a `GoTo` that changes it, as `dt_iop_gui_changed` and `gui_update` reset it; unless the picker is `keep`, darktable's "keep-active";
- its module turns off, since darktable picks only while the module is enabled (`pixelpipe_hb.c:1194-1196`).

**Focus is the client's,** as the desktop-architecture skill has it: the app sends `Off` when the module whose picker is on loses focus (`imageop.c:2414-2416`), and when any module takes focus while the Colour picker's picker is on, unless Restrict scope is on (`imageop.c:2393-2395`). A script that never focuses keeps the picker until it turns it off.

### What a pick writes

**The session applies a module's reading itself,** on the engine thread, as the reading arrives: it calls the module's `pick`, a pure function as `bind` is, and applies what it returns to the edit. Every client gets the same write; no client runs module code.

- **Which readings apply,** darktable's `_record_point_area` rule (`color_picker_proxy.c:76-104`, `:305-327`): the first reading after `On`, the reading after each `Point`, `Area`, `Switch`, `Reset` or `Load`, and after `Tools` its last reading again. A reading measured because an edit changed the operation's input is sent, never applied, so a pick never feeds back on its own write.
- **A pick is a change of source Auto** ([history.md](history.md#merging-consecutive-changes)), its target the picker. A drag's readings are one gesture: each `done: false` region's pick is a gesture's change and the `done: true` region's completes it, one step, merged with a quick repeat on the same picker as a control's change is (decision 78). `On`, `Switch`, `Reset`, `Load` and `Tools` are completed changes. Where a release's region equals the last measured, no new reading is due, and the session completes the gesture with the values already written.
- **It counts as the `Picker` command that asked for it** for `through` and merging: the `Values` after it includes that command, and its step joins the gesture that command continues ([history.md](history.md#merging-consecutive-changes)).
- **A switched module turns on** with the pick, as with any write of its controls (decision 32), and with `On` itself where the module is off, as darktable turns the module on with its picker: that is the first step of a picker that only shows.
- **A pick that gives a value that is not finite,** as `log2` of zero or less where darktable's rule has no floor, writes nothing: the pick is dropped, with no step.
- **A gesture under way is closed as a step** when the picker turns off or fails mid-drag, as any other command on the edit ends a gesture as completed (decision 42).
- **A picked value is stored as darktable stores it:** each module's own rules apply, as in its `color_picker_apply` ([The first pass's pickers](#the-first-passs-pickers)), with no limit to a slider's soft range. The one bound is the parameter's hard limits, darktable's `$MIN` and `$MAX`, the control's range that the session already holds every value to (decisions 21 and 30): a value darktable's rule leaves past them is held at the nearer limit, a hue wrapped onto its turn instead, the same angle. A value beyond the soft range shows as any value set from outside does: the track widens to it, its handle at the end, and the value field shows the true value, as darktable's `dt_bauhaus_slider_set` does ([gui.md](gui.md#how-the-rows-are-laid-out)).
- **Refused writes** write nothing and turn the picker off, reported as `SessionError::Refused` naming the command that asked for the reading. A module's picker cannot turn on while the photo is open for viewing only; the Colour picker's can, and its readings are sent there.
- **A reading measured before a change** that the picker's module has since received is dropped, since that change turned the picker off.

### Tool settings

Some pickers read darktable's own settings, kept in its darktablerc and never in an edit: Exposure's area mode and target lightness, Colour calibration's area mode, channel mixing and target L, h and c (modules rule 12). They become settings in `settings.toml` under `[tools.<module>]`, declared in the settings schema with `owner = "module:<id>"` (decision 43), and their rows name those settings in place of their `Row::ToolSetting` kinds.

- The engine reads no settings file, so the client sends their values with `On` and `Tools`, as the tool's command carries its value (decision 43).
- **Measure mode** writes no control: the pick returns the settings it measured, as darktable's measure mode writes its target sliders and darktablerc (`exposure.c:900-934`), in `Shown::tools`, and the app keeps them in its settings.

## The module side

### Declaring pickers

A module's definition lists its pickers, each darktable's `dt_color_picker_new` call:

```rust
pub struct Picker {
    /// snake_case and unique among the module's controls, preset rows and pickers.
    pub id: &'static str,
    /// The operation whose input, and with `output` output, it reads: one the module owns.
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
    /// The tool settings its pick reads.
    pub tools: &'static [&'static str],
}

pub enum PickerKind { Point, Area, PointArea }
pub enum Space { Operation, Lch, Hsl, Jzczhz }
```

- **`Definition::pickers`** holds them. `Definition::check` holds each to its module: the operation one the module owns, `writes` among its controls, `tools` among its tool settings, a mosaic read only in `Space::Operation` with no denoise (darktable's denoise is 4-channel only), and each picker in exactly one row.
- **Where its button sits,** by its row:
  - **`Quad::Picker(id)`** on a slider row or a choice row: darktable's quad (`dt_color_picker_new` on a bauhaus widget, `color_picker_proxy.c:411-427`), its tooltip darktable's quad tooltip, "pick color from image" where the module sets none;
  - **`Row::Pickers { label, pickers }`**, one line: an optional label at the start and the picker buttons at the row's end, darktable's toggle buttons packed in a box (`:394-410`): AgX's Auto tune levels, RGB levels' black, grey and white, RGB curve's two, Colour calibration's readout row, with its CCT text and swatch when that row comes.
- **`Module::pick`,** a provided method that gives nothing for a picker the module does not run:

```rust
fn pick(
    &self,
    picker: &str,
    reading: &Reading,           // input and, with `output`, output Stats
    values: &Controls,           // every control, the edit's over the defaults
    image: &ImageContext,        // the camera's matrices and the pipe's profiles
    tools: &[(&str, Value)],
    channel: Option<&str>,       // the channel tab the module shows
    last: Option<&Reading>,      // the module's last reading applied
    keys: PickKeys,
) -> Option<Picked>;

pub struct Picked {
    /// Each control it writes with its value, within `writes`.
    pub writes: Controls,
    /// What its panel shows of the reading.
    pub shown: Shown,
}

pub struct Shown {
    /// The input patch: its colour for the screen, as sRGB, and its L, C and h.
    pub patch: Option<Patch>,
    /// Tool settings measure mode set.
    pub tools: Vec<(&'static str, Value)>,
}
```

Pure, as `bind` and `follow` are, so the module's tests drive it with readings and no GPU. **The module's last reading** is darktable's per-module `picked_color`, and RGB levels' `last_picked_color` (`rgblevels.c:80`, `:788-796`), one per module as darktable keeps it, shared by its pickers: the session keeps each module's last reading applied, for the photo, cleared as a photo opens, passes it as `last`, and applies it again on `Tools`. The module's follow writes (decision 27) do not run after a pick: a pick writes what darktable's `color_picker_apply` writes, every control it keeps consistent included, as a preset row's option does.

### The first pass's pickers

Each module's picker, as the reference release declares and applies it. Every one reads its operation's input in the operation's own space unless the table says otherwise.

| Module | Picker id | darktable's widget and kind | Writes, from the reading | Source |
| --- | --- | --- | --- | --- |
| Exposure | `exposure` | Quad on Exposure; area | Correction: Exposure `−log2(Y / Y_target)` of the mean's luminance Y, through the input profile's matrix to XYZ, against the target lightness's, less the camera's bias where Compensate camera exposure is on, plus the highlight preservation bias where that is on, unbounded by darktable; nothing written where it equals the stored value; Black, where it is at or above the new white, set to white − 0.01 (`_exposure_set_black`, `:796-814`). Measure: no control, the target lightness the area gives at the current exposure. The patch: the area as a grey of its L, and "L : 45.2 %" | `exposure.c:866-973`, `:1211-1218`, `:763-779` |
| White balance | `spot` | A toggle in the button bar, and Settings' "from image area"; area; space none, so the mosaic per CFA colour | Each coefficient `(1 / mean) / (1 / green's mean)`, held by darktable's own rule to 0 to 8, a mean of 0.001 or below taken as giving 1, green 1; `preset` from image area | `temperature.c:1948-1969`, `:2082-2095`, `:1816-1822` |
| Colour calibration | `illuminant` | A toggle on the CAT tab's readout row; area | Correction: the illuminant `custom`, its x and y `D50 ∘ LMS / LMS_target` in the adaptation's LMS, each Y-normalised, the target first through the inverse mixer with Take channel mixing into account (`:4286-4410`), and the temperature `_check_if_close_to_daylight` gives for them; no bound of darktable's own. Measure: the target L, h and c the area gives. The patch with its L, h and c | `channelmixerrgb.c:4161-4421`, `:4486-4489` |
| Colour balance RGB | `global_hue`, `shadows_hue`, `midtones_hue`, `highlights_hue` | Quads on the four hues; area | The hue the opposite of the area's, its own with `command`: `atan2` of the mean's Ych in degrees plus the module's 30° shift, plus 180° for the opposite, so −150° to 390°, which darktable stores as it is and Apertrust wraps onto 0° to 360°; the chroma the mean's chroma times its luminance, unbounded by darktable. Ych through the working profile | `colorbalancergb.c:1278-1338`, `:1875-1953` |
| Colour balance RGB | `white_fulcrum`, `grey_fulcrum` | Quads; area | White fulcrum `log2` of the maximum's luminance; contrast grey fulcrum the mean's luminance; neither bounded by darktable | `colorbalancergb.c:1324-1333`, `:2019-2024` |
| AgX | `white`, `black` | Quads on white and black relative exposure; area, denoised | `log2(max(ε, v) / 0.18)` of the channels' largest maximum, or of the smallest minimum, floored at AgX's `_epsilon` first (`agx.c:1110`, `:1126`), scaled by 1 + Dynamic range scaling, held by darktable's own rule to −20 to −0.1 EV for black and 0.1 to 20 EV for white; then the pivot kept at its exposure, held within the new range (`_update_pivot_x`); Curve y gamma left alone, as Keep the pivot on the diagonal leaves it (`agx.c:2657-2690`; `docs/modules/agx.md`, deliberate difference 3; decision 63) | `agx.c:1103-1133`, `:1021-1040`, `:2043-2057` |
| AgX | `range` | Auto tune levels, a toggle on a row of its own; area, denoised | Both at once; Curve y gamma left alone, as above | `agx.c:1135-1160`, `:2072` |
| AgX | `pivot`, `pivot_output` | Quads on Pivot relative exposure and Pivot target output; area, denoised | The pivot at `log2(max(ε, Y) / 0.18)` of the mean's luminance through the working profile, floored at `_epsilon` (`:1178`), held by darktable's rule within black and white relative exposure; with Pivot target output, also the output the curve gave that pivot before, linearised by the curve's gamma, so its brightness stays; Curve y gamma left alone, as above | `agx.c:1171-1220`, `:1823-1840` |
| RGB curve | `show` | A toggle beside the channel tabs; point or area; output too; keep-active | None: the graph draws the input's and the output's mean, minimum and maximum, and the stored samples | `rgbcurve.c:1496-1503`, `:930-1035` |
| RGB curve | `create` | A toggle; area; output too | The shown channel's curve reset, then nodes at the minimum, the mean and the maximum, the mean moved 0.05 up with `command` or down with `shift`; where that makes five nodes, a fourth added midway between the second and the fourth, moved the same way (`:573-578`) | `rgbcurve.c:534-583`, `:1505-1518` |
| RGB levels | `black`, `grey`, `white` | Three toggles in a row; point | For the shown channel, the first channel's mean (`*self->picked_color`, `:786`), applied only when it differs from the module's last one picked (`last`, `:788-796`): black takes it unless it is above grey, then grey − `FLT_EPSILON`; grey takes it only if it lies between black and white, otherwise it is left unchanged; white takes it unless it is below grey, then grey + `FLT_EPSILON`; a step only when a point changed (`:832-837`) | `rgblevels.c:774-838`, `:1068-1077` |

- **AgX's Dynamic range scaling** is the pickers' margin only (`docs/modules/agx.md`, Gaps 2; decision 63): it scales the range a pick writes and writes nothing itself.
- **White balance's Settings** shows "from image area" after a pick, as darktable's combobox shows it at once (`temperature.c:1967`), its stored `preset` `spot`; choosing "from image area" turns the picker on, as darktable's `_preset_tune_callback` does (`temperature.c:1816-1822`). Only in the session it was picked: as the photo opens, and when Undo, Redo or a `GoTo` bring White balance's values back, the label is worked out from the coefficients as darktable's `gui_update` does (`temperature.c:1195-1360`), and a picked balance is "user modified" (`:1338`), the stored `preset` too, in the session's edit, saved only with the next change, never a step of its own, nothing written on open (`docs/modules/white-balance.md`, After a pick).
- **RGB levels' picked point** is stored by darktable's rule above, with no other bound than its parameter's 0 to 1, as `docs/ports/rgblevels.md`'s Tool binding notes state.

### Later modules

| Module | Pickers | Brings |
| --- | --- | --- |
| Filmic RGB | Grey, white and black of the scene, and Auto tune levels on a choice row's quad; area, denoised (`filmicrgb.c:4365-4420`, `:2701-2714`) | A quad on a choice row |
| Colour equalizer | The hue shift's quad, point or area, denoised, in JzCzhz, keep-active, showing the hue on the graph; White level's, area (`colorequal.c:3003-3012`, `:3094`, `:2541-2560`) | `Space::Jzczhz` |
| Blending (Phase 4) | Show colour, point or area, output too, keep-active, and set the range from the area, each in the blend space (`blend_gui.c:2578-2595`, `:1757`) | `Space::Lch` and `Space::Hsl`; a layer's instance in `PickerRef` |
| Colour calibration's colour checker | Its patches' means through the chart's frame | Many boxes in one reading |

**Layers** (Phase 4) add the instance to `PickerRef::Module`: a layer and an instance, as history's entries gain them, the reading taken at that instance's place.

## The picker button

A component of `apertrust-ui`, `PickerButton`, an `IconButton` toggle that draws no engine type: the renderer hands it whether it is on and enabled, and it reports what was pressed.

- **Its look:** Lucide's `pipette`, 18 by 18 in a slider or choice row (before the value or the option, gui.md, How the rows are laid out) and in a `Row::Pickers`; RGB curve's create-curve picker draws darktable's set-values shape (`dtgtk_cairo_paint_colorpicker_set_values`), a pipette over a curve, drawn for Apertrust in Lucide's style.
- **Its states:** off, quiet, the outlined icon in secondary text; under the pointer, the hover plate; on, drawn pressed, `raised` with a `border` hairline, as a toggle with no filled form is (gui.md, Checkboxes, text buttons and icon buttons); disabled, dimmed, on a photo not yet live and, for a module's picker, on a photo open for viewing only, with the tooltip saying why; focused, the focus ring.
- **Its tooltip** is darktable's words for it ("set the exposure adjustment using the selected area", "pick opposite color from image / ctrl+click to pick selected color"), ending, for a picker with both kinds, with darktable's "ctrl+click or right-click to select an area", Cmd-click on the Mac.

| The user | `PickerAction` | Command |
| --- | --- | --- |
| Clicks it while off | `On { area: false }` | `On`: a point, or the picker's one kind |
| Right-clicks it, or Ctrl-clicks (Cmd-click on the Mac) | `On { area: true }` | `On` as an area where the picker has both |
| Clicks it while on | `Off` | `Off` |
| Space or Enter while focused | As a click | |
| Shift+F10 or the Menu key while focused | As a right-click | |

On a Mac, Control-click is the right-click, as everywhere in the app (gui.md, Popups). Turning a picker on moves the keyboard focus to the photo, as darktable gives the centre view the focus (`lib.c:1590`, `:1598`), and opens the picker's module section if it is closed. The renderer gives every quad its picker's state from the mirror's `Picker` fact, and the app sends the tool settings the module's pickers read with `On`.

## On the photo

The central panel draws the active picker and the stored samples over the frame and turns the pointer into commands, from the regions in the photo's coordinates that the events carry. Every mark is drawn with the design system's on-photo overlay, one line and two handles for every mark on the photo ([gui.md, On-photo overlays](gui.md#on-photo-overlays)), as the wireframe's Pickers frame draws them; the app owns the commands.

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
- **The sampled colour** is the sample's frame mean, or the statistic the Colour picker module shows: with the mask's display colour, the only colour the overlay puts on the photo.
- **When:** the active picker while it is on; the stored samples while Show samples on the photo is on, else only the one under the pointer in the list (`darkroom.c:801-829`). Nothing is drawn while the photo is not live (decision 77).
- **The central bar,** while a picker is on, says which picker it is and how to use it, as the wireframe's Pickers frame has it; the module's row shows the patch with what was measured (`Shown::patch`).

**The central bar's words,** Fluent messages in `ui.ftl`, Ctrl written Cmd on the Mac by the app's modifier rule:

| Part | Message | British English |
| --- | --- | --- |
| Its start: the module and the control or picker | `picker-bar-name` | "{ $module } · { $picker }": "Colour calibration · Illuminant", "RGB levels · Grey point" |
| An area | `picker-bar-area` | "Drag an area on the photo. Ctrl-click switches to a point. Esc ends." |
| A point, after the picker's own sentence | `picker-bar-point` | "{ $what } Ctrl-click switches to an area. Esc ends." |

- **The picker's own sentence** names what to pick, as its row's tooltip does, as Colour calibration's "Pick the illuminant from a neutral area of the photo": a point picker's `module-<module>-picker-<picker>-what` message. A picker starts as a point on a plain click where it is a point picker, as RGB levels' black, grey and white (`DT_COLOR_PICKER_POINT`, `rgblevels.c:1069`), or a point-and-area one, as RGB curve's show picker and the Colour picker; the example is RGB levels' grey picker, "RGB levels · Grey point" with "Click or drag the point on a neutral grey." An area picker switched to a point by Ctrl-click, as White balance's, which always starts as an area, has no sentence of its own and says "Click or drag the point on the photo." (`picker-bar-point-what`). The words are the same whatever kinds the picker has, since darktable's Ctrl-click on the photo switches the kind of every picker (`darkroom.c:4019-4034`).
- **The picker's name** is its control's label where it sits in a quad ("Exposure"), else its row's or its button's words ("Illuminant", "Auto tune levels", "Grey point"), `module-<module>-picker-<picker>-label`.
- **The Colour picker module's own picker** names "Colour picker" alone.

**The pointer,** darktable's `button_pressed`, `mouse_moved` and `button_released` (`darkroom.c:3815-3948`, `:3984-4140`), while a picker is on:

| The user | Command |
| --- | --- |
| Presses with a point picker | `Point { at, done: false }`; a drag moves it, each step `done: false`; the release `done: true` |
| Presses within 6 points of an area's corner | That corner taken, the opposite one held; the drag sends `Area` from the held corner to the pointer, the release `done: true` |
| Presses elsewhere with an area | A new area 2 % of the photo's width each way of the press, as tall on screen, the press held; the drag sends `Area` from the press to the pointer, at least one frame pixel each way, the release `done: true` |
| Ctrl-clicks (Cmd-click on the Mac) | `Switch` |
| Right-clicks, with Show samples on the photo on, on a stored sample of a kind the picker takes | `Load { id }`, within `max(26, round(3 × zoom))` points of a point sample, `zoom` the points a frame pixel spans, so the reach grows with the zoom (`darkroom.c:4108-4113`), or inside an area sample |
| Right-clicks otherwise, with an area | `Reset` |

- The photo does not pan while a picker takes its drags; in a zoomed view, dragging past the view's edge scrolls it, as darktable's does (`darkroom.c:3895-3902`). The pointer is the move cursor during an area's drag.
- **What is drawn follows the pointer at once:** the app draws the area or point it sent until a reading includes it, as the panel shows a pending value (gui.md, The renderer), so the drag never waits for a reading.

**Keys,** with the picker's point or area focused on the photo: Apertrust's own, darktable having none there, following the quality bar's "everything can be reached by keyboard". While its node has the focus, it takes these keys before the app's actions, so the arrows, Space and Backspace move the picker rather than going to the next or previous photo (gui.md, Next Photo and Previous Photo; Who owns the keys):

| Key | Does |
| --- | --- |
| Arrow keys | Move the point, or the whole area, one frame pixel; Shift ten |
| Alt+arrow keys | Move an area's right or bottom edge one frame pixel, growing or shrinking it; Shift+Alt ten |
| Space | `Switch`, point to area or back |
| Delete or Backspace | `Reset`, the picker back to its default point or area |
| Esc | `Off`, the focus back to the picker's button |

Each press is a completed change, so a held arrow's repeats merge into one step, as a slider's do (decision 78).

## Accessibility

- **The button** is a toggle button to AccessKit, named by its full name ("Exposure: pick from the photo", the module's name and the picker's words), pressed while on, its keys as its shortcuts, its tooltip its description.
- **The picker on the photo** is a node of its own while a picker is on, a slider-like group named "Picker area, Exposure" or "Picker point, Colour picker", its value the region in percent of the photo ("point at 52 %, 40 %", "area from 10 %, 12 % to 30 %, 25 %"), with Increment and Decrement moving it along x as → and ← do, the custom actions Move up and Move down moving it along y, and its keys as described.
- **The reading is read out** through a polite live region, once a reading follows a move or a pick and the pointer or the keys have rested half a second: the Colour picker's value in its colour mode ("mean RGB 128, 120, 99"), and for a module's pick what it changed ("Exposure +0.84 EV"), from the `Values` that follow.
- **The Colour picker module's rows** are labelled rows: each sample's patch a toggle button "Lock sample 2", its value text read with its mode, its remove button "Remove sample 2", the number its place in the list ([Commands](#commands), Ids and numbers).
- **Colour is never the only cue:** the swatches carry their values as text, and the area's lines are drawn light over dark, visible on any photo.

## The Colour picker module

darktable's color picker utility (`src/libs/colorpicker.c`, in the darkroom's left panel with its history and snapshots, position 800), as the wireframe's Pickers frame has it: a section of the History tab, under the Scopes pinned there as in every editing tab, closed by default and kept open or closed with the other sections (decision 38). Its rows, darktable's (`:696-865`) in the frame's order and words, top to bottom:

1. **The header,** the section's, with the picker button in it: darktable's picker button, its kind chosen as darktable's is, a click for a point, right-click or Ctrl-click (Cmd-click on the Mac) for an area (`:775-779`), its tooltip darktable's: "Turn on colour picker / Ctrl+click or right-click to select an area / Ctrl+click on the photo to switch between point and area".
2. **One row of two choices:** the statistic, Mean, Min or Max, then the colour mode, darktable's eight: RGB, RGB %, Lab, LCh, HSL, HSV, Hex and None (`:754-773`).
3. **The live sample:** a patch 52 points square and its values beside it, one channel per line, in IBM Plex Mono and selectable (`:786-815`). A click on the patch shows and hides darktable's large patch above the rows, hidden by default (`ui_last/colorpicker_large`, `:272-283`).
4. **Add sample,** a plate button (`Button`), enabled only while a picker is on (`:816-821`, `:268-269`).
5. **The stored samples,** numbered by their place in the list, from 1, one row each: its number, a small patch (a click locks it, a lock drawn on it; a right-click loads it into the active picker, or stores the picker's place into it, a store mark drawn on it while loaded), its values and × to remove it, in a `section_list` (`:597-673`).
6. **Two checkboxes:** Show samples on the photo, darktable's "display samples on image/vectorscope" (`:830-840`), and Restrict the scopes to the selection, darktable's "restrict scope to selection" (`:842-853`).

- **Values,** darktable's `_update_sample_label` (`:188-259`) from the reading's statistic: RGB, each channel of `rgb` times 255 rounded; RGB %, times 100 with one decimal; Lab, L clamped to 0 to 100 and two decimals; LCh, from Lab, h in degrees; HSL and HSV, from `rgb`, hue in degrees and the rest in percent; Hex, `0x` and the three bytes clamped; None, "◎". The patch is the statistic's `rgb` as the screen shows it.
- **The tooltip** of a value, darktable's (`:356-425`): a table of mean, min and max as RGB with their swatches and hex, Lab, and the colour's name from the mean's LCh (`Lch_to_color_name`, `src/common/color_vocabulary.c`, restated in `apertrust-color`).
- **Hovering a sample's row** marks it on the photo, as darktable's `selected_sample` (`:476-506`).
- **Restrict the scopes to the selection** makes the scopes count only the Colour picker's point or area while its picker is on, as darktable's histogram does (`src/libs/histogram.c:115-160`): `Command::Scopes` gains the restriction, and the scopes' kernel counts within the frame's rectangle it maps to. With it on, giving a module focus leaves the Colour picker's picker on.
- **Show samples on the photo** also marks each stored sample's mean on the vectorscope, as darktable's checkbox does (`src/libs/scopes/vectorscope.c:622-640`), placed on the plane by the scopes' own `Plane` (decision 19), and RGB curve's graph draws the stored samples, as `rgbcurve.c:936-990` does.
- **Its reset** in the header sends `SamplesCommand::Clear` and sets Mean, RGB and Show samples off (`gui_reset`).
- **Its settings,** darktable's `ui_last/colorpicker_*`: the statistic, the colour mode, the large patch, Show samples and Restrict the scopes, in `settings.toml` under `[colour_picker]`, declared in the settings schema; the stored samples last only for the session, as darktable's do.
- **Actions,** darktable's: Pick colour (the button, a toggle), Add sample, Show samples and Restrict the scopes, each bindable in Settings › Shortcuts, with no default keys, as darktable has none.

## Against darktable

| darktable 5.6.2 | Apertrust | Why |
| --- | --- | --- |
| Measured in the preview pipe, downscaled throughout, the mosaic included | In the fitted viewport pipe: after the downscale at the viewport's size, in the raw stage at full resolution | Decision 63: the export's processing at the view's scale; White balance's mean is the photosites the export reads |
| Measured on the CPU, or copied from the GPU to the host for it (`_pixelpipe_picker_cl`, `pixelpipe_hb.c:1008-1106`) | Reduced on the GPU, 48 bytes a region read back | GPU-resident pipeline (decision 8) |
| A picker's reading applied by the GUI's callback | Applied by the session through `Module::pick`, the same for every client | Every client through the session protocol (decision 58) |
| Each applied reading its own undo record (`dt_dev_add_history_item` with no target) | A drag's readings one gesture, one step of source Auto, merged as a control's | Decision 78 |
| No keys on the photo for a picker | With its point or area focused: arrows move it, Alt+arrows resize an area, Space switches point and area, Delete resets it, Esc turns it off; the region a node for screen readers | The quality bar: everything can be reached by keyboard |
| The utility in the darkroom's left panel, with the history | A section of the History tab, under its Scopes | One side panel (decision 38), the History tab holding what darktable's left panel holds |
| A picked value stored as its module's rule gives it, past a parameter's `$MIN` or `$MAX` where the rule has no bound | Held at the nearer hard limit | Every stored value within its parameter's range (decisions 21 and 30) |
| Colour balance RGB's picked hue stored from −150° to 390° | Wrapped onto 0° to 360°, the same angle | Within the hue's hard limits, the render unchanged |
| The Colour picker's button at the end of its statistic and colour mode row (`libs/colorpicker.c:772-775`) | In the section's header | The wireframe's layout |
| "Live samples", "display samples on image/vectorscope", "restrict scope to selection" | Stored samples, numbered; Show samples on the photo; Restrict the scopes to the selection | The wireframe's words; the behaviour darktable's, the vectorscope's marks included |

## Where the code lives

| Part | Where |
| --- | --- |
| `Picker`, `PickerKind`, `Space`, `Quad::Picker(id)`, `Row::Pickers`, `Module::pick`, `Reading`, `Stats`, `Picked`, `Shown`, `Region`, `PickKeys`; `Definition::check`'s rules | `apertrust-modules`, its `definition` and a `pick` module |
| Each module's pickers and `pick` | `apertrust-modules`, the module's file |
| The reduction kernel | `crates/engine/slang/picker.slang`, its WGSL generated beside it |
| The active picker, stored samples, regions mapped through the geometry, keys, sample-only renders, applying a pick, the events | `apertrust-engine`'s `picker` module and the session |
| The staging pair | `crates/engine/src/readback.rs`, shared with the scopes and readouts |
| The on-photo overlay's primitives | `apertrust-ui`'s `overlay` ([gui.md](gui.md#on-photo-overlays)) |
| `PickerButton`, the picker's marks, the Colour picker's rows | `apertrust-ui`'s components, in the gallery |
| The quads and rows drawn, the overlay's commands, focus, the Colour picker section | The app's `modules.rs`, central panel and a `colour_picker` module |
| The colour's name | `apertrust-color` |

## Tests

Planned, with each part:

- **The kernel:** on synthetic RGBA and mosaic buffers, every box's mean, minimum and maximum equal to a CPU restatement of darktable's `_color_picker_work_4ch` and `_work_1ch` with the same tree (`apertrust-golden`'s helpers), within 1 ulp on the mean; boxes of 1 pixel, a row, a column, odd sizes, the whole buffer, at the buffer's edges; Bayer and X-Trans colours; denoise against `blur_2D_Bspline` restated; a buffer in row strips equal to the same buffer whole; each space against darktable's conversions restated, the hue's fourth channel included.
- **Regions:** the default point and area; corners mapped back and forward through orientation's `flip` (the first geometry) as darktable's sort-and-average gives them, then through each geometry as it lands; an area outside the image measuring nothing; a point the pixel it falls in.
- **The session:** one picker on at a time; `On` taking the last place or the default; `Switch` and `Reset`; a reading applied on `On` and on each move, never after an upstream edit; a drag one step of source Auto, its release completing it, a release with no move completing it too; the picker off after its module changes, after Undo, after another photo opens, kept with `keep`; a switched module turned on by `On`; each module's own bounds, nothing else but the hard limits, a hue wrapped; `On` refused before live and, for a module's picker, on a view-only photo; a non-finite pick dropped with no step; a gesture closed as a step when the picker turns off mid-drag; `On`'s capture into the edit cache and the first moved area after a rebuild; a sample-only render publishing no frame and equal to a full one's reading; stored samples added, locked, loaded, stored into and removed; the events' order against `Frame` and `Values`; a refused reduction turning pickers off and the frame still published.
- **Each module's pick:** pure tests from readings to writes, each against darktable's `color_picker_apply` restated, the reading darktable's own on a fixed area where one was recorded: E50 for Exposure, W-rows for White balance, and each module's page's rows.
- **By hand against darktable:** each module's pick on the corpus' 7R V and X-T5 at the same areas in darktable 5.6.2's GUI, the values written recorded in the module's page's Results, with no bound, as the scale differs ([The scale](#the-scale)).
- **The app:** the button's states, keys and AccessKit node; with the picker's region focused, the arrows, Space, Delete, Backspace and Esc reaching the picker and not Next Photo or Previous Photo, through `egui_kittest`; the picker's marks in snapshots, light and dark; the pointer's table and the keys' table through `egui_kittest`, each emitting its command; the Colour picker's rows, values in every colour mode, and their snapshots.
- **Measured:** the costs and budgets of [Cost](#cost) on the M4, in 79.1 and 79.3.

## The plan

Phase 2 ([roadmap.md](roadmap.md#pickers)), one pull request each, each reviewed before the next that needs it. 79.1, 79.2 and 79.4 can start at once, 79.4 before any mark is drawn on the photo; switched modules (decision 32) come before the pickers of modules that are switched; history's 9.1 brings a pick's source, Auto, and until then a pick is a completed change as any.

| # | Delivers | Touches | Out of scope |
| --- | --- | --- | --- |
| 79.1 | The reduction kernel, `picker_rgba` and `picker_combine`, on the frame; `Region`, `Stats` and `PickKeys` in the modules crate; `PickerCommand` for `PickerRef::ColourPicker` alone, with `On`, `Point`, `Area`, `Switch`, `Reset` and `Off`; regions mapped through orientation; `Event::Picker` with its frame part; the staging pair shared as `readback.rs`; the frame samples' cost measured at 2400×1600 on the M4 | modules, engine | module pickers, stored samples, the app |
| 79.2 | The modules crate's side: `Picker`, `Definition::pickers`, `Quad::Picker(id)`, `Row::Pickers`, `Module::pick` with its `channel` and `last`, `Picked` and `Shown`, `check`'s rules, `testing::Sample`'s pickers; the built definitions' quads naming their pickers | modules | the engine, every real module's pick |
| 79.3 | The engine's side: the reduction at an operation's input and output, `picker_mosaic`, denoise; `On`'s capture and the operation as the edited instance; sample-only renders through `Pipeline::writes`; applying a pick as a gesture, the module's last reading; turning off; `Tools` and `Channel`; the moved area's budgets measured on the M4 | engine | other spaces, the app |
| 79.4 | Built: the design system's on-photo overlay primitives ([gui.md](gui.md#on-photo-overlays)): the line in its three weights, the square and circle handles, the sampled-colour centre and the focus ring on the photo, in points over a photo's rectangle, in the gallery with snapshots over light, dark and busy photos | ui | any tool's marks |
| 79.5 | `PickerButton` with its states, keys and AccessKit, and the picker's marks drawn with 79.4's primitives, in the gallery with snapshots | ui | the app |
| 79.6 | The app: quads and `Row::Pickers` drawn with their state, the marks over the photo, the central bar's words and their messages, the pointer's and the keys' tables, the keys taken before the app's actions, focus, the live region; driven by `testing::Sample` | app | real modules |
| 79.7 | Exposure's picker: its pick, Area exposure mapping's settings in the settings schema, the patches and the L readout; E50 | modules, app, settings schema | Auto |
| 79.8 | White balance's picker: `spot` in Settings and its pick, the label "user modified" when the module is shown again; its rows on the module's page | modules, app | the button bar |
| 79.9 | Colour calibration's picker: the readout row with CCT and the swatch, Area colour mapping's settings, its pick in both modes | modules, app | the colour checker |
| 79.10 | Colour balance RGB's six pickers, `command` for the colour itself | modules | the mask display |
| 79.11 | AgX's five pickers, Dynamic range scaling as their margin, Auto tune levels as a `Row::Pickers` | modules | Read exposure, Show curve |
| 79.12 | RGB curve's two pickers and its graph's readings, with RGB curve | modules, app | blending |
| 79.13 | RGB levels' three pickers, with RGB levels | modules, app | Auto region |
| 79.14 | `SamplesCommand` and `Event::Samples`: stored samples measured on the frame, locked, loaded, stored into, removed and cleared | engine | the app |
| 79.15 | Restrict the scopes to the selection: `Command::Scopes` gaining the Colour picker's point or area, the scopes' kernel counting within it | engine | the app |
| 79.16 | The Colour picker section in the History tab: the header's picker button, the statistic and colour mode, the live sample with its 52-point patch, Add sample, numbered stored samples, Show samples on the photo with the vectorscope's and RGB curve's marks, Restrict the scopes, its settings and actions | ui, app, settings schema | blending |

**Later, with their modules:** Filmic RGB's pickers and a quad on a choice row (Phase 3); the colour equalizer's, with `Space::Jzczhz` (Phase 3); Colour calibration's colour checker, many boxes in one reading (Phase 3); masks' overlays on 79.4's primitives and blending's pickers, with `Space::Lch` and `Space::Hsl` and a layer's instance in `PickerRef` (Phase 4); picking by coordinates through MCP (Phase 6).
