# Input colour profile

Tool `colorin`, version 1. It works on the background only and has no Auto.

Its header's tooltip says what it is for (`tool-colorin-description`, [gui.md](../gui.md#the-headers-tooltip)): "Takes the camera’s colours into the colour space every later tool works in. Usually left at the camera’s own matrix."

It is darktable's input color profile in the first pass (decision 2). darktable's page has no tabs, sections or presets, and shows three controls in its `gui_init`'s order, Input profile, Working profile and Gamut clipping, here under their labels in British English (decision 29). Its tab is Edit's Raw, after Demosaic and before Output colour profile (decision 38; [README.md](README.md), Raw). darktable's scene-referred preset puts it in no group; the darktable workspace gives it Colour, last, the group darktable's "modules: all" gives it ([README.md](README.md), The darktable workspace).

It drives one operation, `colorin` (camera RGB 28, [../ports/colorin.md](../ports/colorin.md)), which every photo runs, so it adds no operation. It has no first step, and at its defaults it sets nothing and leaves the plan as it was. It owns every parameter of the operation. darktable has it on by default with no on/off button, so a new photo's history holds its default step, after Demosaic's and before Output colour profile's ([history.md](../history.md#original-and-the-default-steps)).

**Status:** built, in `crates/tools/src/input_colour_profile.rs`. The app draws it from its definition (`docs/gui.md`, Tool panels from definitions) in Edit's Raw tab.

- Its rows are tests in `crates/tools/tests/input_colour_profile.rs` and the tool's source, and the panel's in `crates/app/src/tools.rs`; [Results](#results) gives what they measured.
- Every row is exact or darktable's own default, so none has a proposed tolerance; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

| Control | Parameter | Shown as | Default | Ported |
| --- | --- | --- | --- | --- |
| Input profile (`type`) | `type` | a list: the photo's own matrix, then darktable's fifteen built-in input profiles (below) | the photo's, colorin's and darktable's `reload_defaults`: the embedded matrix on a DNG whose image context holds its own, the standard colour matrix on another colour raw, linear Rec2020 RGB otherwise | the photo's own matrix, linear Rec709 RGB and linear Rec2020 RGB |
| Working profile (`type_work`) | `type_work` | a list of darktable's eleven working profiles (below) | linear Rec2020 RGB, darktable's `$DEFAULT` and the operation's | linear Rec2020 RGB alone; unavailable |
| Gamut clipping (`normalize`) | `normalize` | a list: Off, sRGB, Adobe RGB (compatible), linear Rec709 RGB, linear Rec2020 RGB, `dt_iop_color_normalize_t`'s `$DESCRIPTION`s | Off, darktable's `$DEFAULT` | Off alone; unavailable |

**Input profile** lists the photo's own matrix first, as darktable's `update_profile_list` does:

- **Embedded matrix,** on a DNG whose image context holds its own matrix (`Camera::embedded_matrix`), darktable's `d65_color_matrix`.
- **Standard colour matrix,** on another raw with a matrix, the camera table's: rawler's where darktable's is its own (decision 23). It is listed on a colour raw with no matrix too, where it is the photo's default, which colorin cannot render.

Then darktable's built-in input profiles, in the order `dt_colorspaces_init` gives them their `in_pos`: sRGB, Adobe RGB (compatible), linear Rec709 RGB, Rec709 RGB, linear Rec2020 RGB, PQ Rec2020 RGB, HLG Rec2020 RGB, PQ P3 RGB, HLG P3 RGB, Display P3 RGB, linear ProPhoto RGB, linear XYZ, Lab, linear infrared BGR and BRG (for testing). Of them colorin ports linear Rec709 RGB and linear Rec2020 RGB; the thirteen others are refused as commands and drawn disabled, "Coming in a later version" ([Gaps](#gaps), 3).

**Working profile** lists darktable's working profiles in their `work_pos` order: sRGB (web-safe), Adobe RGB (compatible), linear Rec709 RGB, Rec709 RGB, linear Rec2020 RGB, PQ Rec2020 RGB, HLG Rec2020 RGB, PQ P3 RGB, HLG P3 RGB, Display P3 RGB and linear ProPhoto RGB. Every operation after colorin assumes linear Rec2020, so it is the only one ported.

**Gamut clipping** is not ported: Off alone.

Working profile and Gamut clipping each have one value an edit can choose, their default, so they change nothing on any photo: each is unavailable (`ProcessingTool::available`, decision 30), its box drawn disabled at it, as Output colour profile's Export profile is.

Each option's id is the operation's choice of that name (`apertrust_color::PROFILE_TYPES` for the two profiles, colorin's `normalize` choices for gamut clipping), and the binding writes the operation's choice by its name.

**Commands.**

- A script sends `SetControl { tool: "colorin", control: "type", value: … }`, and `apertrust-cli edit --set colorin.type=lin_rec709` does the same; `--set colorin.type=srgb` is refused as not ported, and `--set colorin.type=embedded_matrix` on a raw that is not such a DNG as not listed (decision 30).
- `--param colorin.type=…`, `colorin.type_work` and `colorin.normalize` are refused, since the tool owns them, and `colorin.intent` and `colorin.blue_mapping` as fixed by the tool.
- An edit that holds one of colorin's parameters as an operation parameter, as an edit from before the tool could, through `--param` or `Command::Set`, loses it with a warning when it is read ("the tool colorin owns it now; left out"), and stays editable.

**Not shown** (rule 12): the intent (`intent`) and blue mapping (`blue_mapping`), which darktable's page has no widget for. The tool owns them at the operation's, perceptual, which a matrix profile ignores, and off. **Presets:** darktable's input color profile has none.

The words are darktable's labels and option names from its catalogue, in British English, the names capitalised as the other tools' are; Ukrainian and Polish from darktable's `uk.po` and `pl.po` at the tag. Gamut clipping's promise is darktable's tooltip. darktable's tooltips for Input profile and Working profile name the folders it loads ICC profiles from, which Apertrust does not read, so their promises are Apertrust's own, saying what each does; their Ukrainian and Polish are Apertrust's drafts.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since every photo runs colorin; then the binding, what an edit cannot choose, what the photo lists, darktable's match and layers. No control's unit is Apertrust's.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| I0 | Every control at its default | Untouched, written at the photo's default, or kept | `bind`; the operations and parameters planned; the frame | `bind` returns nothing; the operations and parameters those of the pipeline without the tool; the frame no edit's | Exact | The small Bayer raw and the same as a DNG with its own matrix, through the export and the viewport; every corpus raw |
| I1 | Input profile, moved and reset | Each profile the photo lists that an edit can choose, then reset by `ResetControl` and by `ResetTool` | The profile colorin renders; the frame | The profile chosen is the one rendered, the frame colorin's with that profile set directly, no edit's for the photo's own matrix and another for each linear one; once reset, no edit's frame and keys | Exact | The small raw and its DNG through the export and the viewport; the 90D through the viewport and the export |
| I2 | The binding | Each option of each choice | `bind`'s settings | Each option off the photo's default writes colorin's parameter as the operation's choice of its name, and nothing else; the default writes nothing | Exact | CPU, a colour raw's context, a DNG's, a linear DNG's, a raw with no colour filters and an image that is not a raw |
| I3 | What an edit cannot choose | Each option not ported; the camera matrix the photo does not list; `--param` on each parameter | The edit; the definition | Each refused, the edit unchanged: thirteen input profiles, ten working profiles and four gamut clippings as not ported, the other camera matrix as not listed; the three choices refused at operation level as owned, the intent and blue mapping as fixed | Exact | CPU |
| I4 | What the photo lists | None | Input profile's options; each control's availability | The embedded matrix on a DNG with its own, a linear one among them, the standard matrix on another raw with a matrix or colour filters, neither on a raw with no colour filters and no matrix or an image that is not a raw, then the fifteen built-in profiles; Input profile available, Working profile and Gamut clipping not, on every photo | Exact | CPU, the contexts of I2 and a linear DNG with its own matrix |
| I5 | The darktable match | None | The definition's lists; the profile the tool shows and the one an unedited photo, and each untouched edit, renders | darktable's lists in its order with its preferences at their defaults; the profile shown is colorin's default for the photo and the one rendered, darktable's on every colour raw, which `apertrust-golden pipeline` holds to darktable-cli's export; on a raw with no colour filters, both linear Rec2020 where darktable chooses linear Rec709, a grey image rendering alike ([Gaps](#gaps), 5) | Exact | CPU; the small raw and its DNG; every corpus raw |
| I6 | On a layer | | The definition | Background only: never offered on a layer, as colorin takes the whole image into the working space before every layer | Exact | CPU |

I0 is the check that the tool changes nothing it renders: an untouched edit gives the very operations and parameters the built-in tools without it give, so the same plan, cache keys and frame as before the tool. I5 is how the tool matches darktable: the whole-pipeline exit test compares each corpus raw at its defaults with darktable-cli's export, and the tool cannot move it.

## Results

Measured on the Apple M4 (Metal) by `crates/tools/tests/input_colour_profile.rs` and the tool's source. The small-image and CPU rows run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| I0 | Holds, exactly: `bind` sets nothing at the defaults, the operations and parameters are those of the pipeline without the tool, and the frames are byte-identical to no edit on the small raw and its DNG through the export and the viewport. On all 31 corpus raws the pipeline takes, the same operations and parameters as without the tool |
| I1 | Holds, exactly, on the small raw and its DNG through the export and the viewport, and on the 90D through the viewport and the export |
| I2 | Holds for every option of the three choices on the five contexts |
| I3 | Holds, exactly |
| I4 | Holds on the five contexts |
| I5 | Holds: the lists are darktable's in its order; the small raw, its DNG and all 31 corpus raws render colorin at the tool's defaults, the DNGs with their own matrix at the embedded matrix |
| I6 | Holds: the definition works on the background only, with no Auto |

## In the panel

- **No tabs:** Input profile, Working profile and Gamut clipping, as darktable's page has them (`input_colour_profile_draws_darktable_s_three_lists`). Input profile opens on the photo's profiles, the unported ones drawn disabled with "Coming in a later version"; Working profile and Gamut clipping are drawn disabled at their defaults and do not open.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **darktable's own camera tables.** darktable lists the enhanced, vendor and alternate colour matrices for the cameras its tables hold, and the standard colour matrix beside a DNG's embedded one where its camera table holds that camera. Apertrust has neither those tables nor a camera-table matrix for a DNG, so no photo could list them: the definition leaves the three out, and a DNG with its own matrix lists only it.
2. **Embedded ICC profiles and ICC files.** darktable lists a file's embedded ICC profile, for a JPEG, TIFF or PNG it opens, and the ICC files in its `color/in` and `color/out` folders after the built-in profiles, chosen by `type` file and their `filename`. Apertrust opens raws and reads no ICC file, and colorin has no file name parameters (`docs/ports/colorin.md`, Parameters): they are left out, and the promises say what each control does in place of darktable's tooltips.
3. **The input profiles colorin does not port:** sRGB, Adobe RGB (compatible), Rec709 RGB, the PQ and HLG profiles, Display P3 RGB, linear ProPhoto RGB, linear XYZ, Lab, linear infrared BGR and BRG (for testing), refused and drawn disabled.
4. **Working profiles other than linear Rec2020,** and gamut clipping: not ported, refused, and drawn disabled until each is.
5. **A raw with no colour filters:** darktable's default for a monochrome raw is linear Rec709 RGB; colorin's, and so the tool's, is linear Rec2020 RGB. Both render a grey image alike, the two profiles sharing D65 as white.
6. **The embedded ICC profile's tooltip,** which darktable writes with the profile's name, version and maker: there is no embedded ICC profile to describe.
7. **A four-colour raw** (darktable's Bayer4): darktable's default is linear Rec2020 RGB, the image converted to it before colorin, and it lists no standard colour matrix. Apertrust's demosaic does not take such a raw yet, so the tool shows the standard colour matrix there, colorin's default for a colour raw, which nothing renders.
8. **Exposure's picker** reads its area through colorin's default input profile for the photo (`exposure::input_profile`), not the one Input profile holds: once a linear profile is chosen, a pick reads the area through the camera's matrix still ([exposure.md](exposure.md#the-picker)).
9. **The tool's tooltip,** darktable's "convert any RGB input to pipeline reference RGB using color profiles to remap RGB values": a definition implies no message for the tool's own tooltip.

## Deliberate differences

- **The standard colour matrix on a colour raw with no matrix:** listed, though darktable lists it only where its camera table has a matrix, since it is the photo's default there, colorin's, and a choice lists the photo's default (decision 30). colorin cannot render such a raw.
- **A DNG's standard colour matrix:** an edit from another photo holding the standard colour matrix renders a DNG with its own matrix through the image context's one matrix, the DNG's, where darktable renders its camera table's, or linear Rec709 RGB where its table lacks the camera. Pasted the other way, the embedded matrix on a raw without one renders the standard matrix, as darktable's `commit_params` falls back to it.

## Design choices

- **Control ids:** `type`, `type_work` and `normalize`, the operation's parameters and darktable's params fields, so `--set` and scripts name what darktable's history names.
- **Option ids:** the operation's choices by name, so the binding needs no table of its own.
- **The lists:** darktable's, in its order, less what no photo in Apertrust can list ([Gaps](#gaps), 1 and 2).
- **Input profile's default** is colorin's for the photo, which this crate cannot call, so it repeats colorin's rule; I5 holds the two together.
- **The promises:** Input profile "The colour profile the camera's colours are read in.", Working profile "The colour space every tool after this one works in.", since darktable's tooltips name folders Apertrust does not read and would promise files; Gamut clipping's is darktable's tooltip, "Confine Lab values to gamut of RGB colour space."
- **Rows:** I0 to I6. No first step, since every photo runs colorin, and no order row, since only Input profile can move.
- **The panel's test:** in `crates/app/src/tools.rs`, beside every other tool's, with its snapshots, the page and its Input profile list open, in `crates/app/tests/snapshots/input-colour-profile.png` and `input-colour-profile-list.png`.
- **Row ids:** I, for Input colour profile.
- **In `TOOLS`:** after Demosaic, before AgX, its place in the pipeline among the Raw tab's tools.
