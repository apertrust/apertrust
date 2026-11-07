# Output colour profile

Module `colorout`, version 1. It works on the background only and has no Auto.

It is darktable's output color profile in the first pass (decision 2). darktable's page has no tabs, sections or presets, and shows one control, Export profile, here under its label in British English (decision 29). darktable's scene-referred preset puts it in no group, so it takes Colour, first, the group darktable's other layouts give it (decision 38; [README.md](README.md), In no group).

It drives one operation, `colorout` (display 70, [../ports/colorout.md](../ports/colorout.md)), which every photo runs, so it adds no operation. It has no first step, and at its default it sets nothing and leaves the plan as it was. It owns both parameters of the operation.

**Status:** built, in `crates/modules/src/output_colour_profile.rs`. The app draws it in the Colour tab from its definition (`docs/gui.md`, Module panels from definitions), first, before Colour balance RGB.

- Its rows are tests in `crates/modules/tests/output_colour_profile.rs` and the module's source, and the panel's in `crates/app/src/modules.rs`; [Results](#results) gives what they measured.
- Every row is exact or darktable's own default, so none has a proposed tolerance; the [design choices](#design-choices) are settled.
- Measures and test images are in [README.md](README.md).

## Controls

| Control | Parameter | Shown as | Default | Ported |
| --- | --- | --- | --- | --- |
| Export profile (`type`) | `type` | a list of darktable's twelve export profiles (below) | sRGB (web-safe), darktable's `$DEFAULT` and the operation's, on every photo | sRGB; linear Rec709 and linear Rec2020 ported by the operation but held back (below); unavailable |

**The list** is darktable's export profiles, in the order `dt_colorspaces_init` numbers them (`src/common/colorspaces.c`, their `out_pos`), under darktable's names: sRGB (web-safe), Adobe RGB (compatible), Linear Rec709 RGB, Rec709 RGB, Linear Rec2020 RGB, PQ Rec2020 RGB, HLG Rec2020 RGB, PQ P3 RGB, HLG P3 RGB, Display P3 RGB, Linear ProPhoto RGB and BRG (for testing). Each option's id is the operation's choice of that profile (`apertrust_color::PROFILE_TYPES`: `srgb`, `adobe_rgb`, `lin_rec709`, `rec709`, `lin_rec2020`, `pq_rec2020`, `hlg_rec2020`, `pq_p3`, `hlg_p3`, `display_p3`, `prophoto_rgb`, `brg`), and the binding writes the operation's choice by its name.

Linear XYZ and Lab, which darktable lists only while its `allow_lab_output` preference is on, off by default, are left out (rule 12). darktable lists every built-in profile on every photo, so the list does not follow the photo.

**sRGB is the only profile an edit can choose.** colorout ports sRGB, linear Rec709 and linear Rec2020 (`docs/ports/colorout.md`). But the viewport draws colorout's output as sRGB, and the CLI tags its PNG sRGB, so a linear profile would show and save wrong. Decision 26 makes the display path's conversion from the edit's profile the condition for letting an edit choose another profile, and that conversion is not built ([Gaps](#gaps), 1).

So the two linear profiles are refused as commands, as the nine the operation does not port are. The control, whose one value is its default, can change nothing on any photo: it is unavailable (`Module::available`, decision 30), its box drawn disabled at sRGB (web-safe), as Raw black/white point's flat field correction is.

A disabled box does not open, so its list is not seen until the control is available. Then every profile still refused shows in it drawn disabled, "Coming in a later version". The binding already writes every profile by its name ([O2](#invariants)), so once the display converts, a profile becomes choosable by leaving the definition's `unported` list, and the control becomes available.

**Commands.**

- A script sends `SetControl { module: "colorout", control: "type", value: … }`, and `apertrust-cli edit --set colorout.type=srgb` does the same; `--set colorout.type=lin_rec2020` is refused as not ported.
- `--param colorout.type=…` is refused, since the module owns it, and `--param colorout.intent=…` as fixed by the module.
- An edit holding a `colorout` value at operation level, under `param:`, loses it with a warning when it is read, and it is gone from the file after its next save.

**Not shown** (rule 12): Output intent (`intent`), which darktable shows only while its preference to always use LittleCMS 2 for export is on, off by default (`gui_init`, `_preference_changed`). The module owns it at perceptual, the operation's, which a matrix profile ignores, and never sets it, so no edit, script or `--param` moves it. **Presets:** darktable's output color profile has none.

The words are darktable's label and option names from its catalogue, in British English, the names capitalised as the other modules' are; Ukrainian and Polish from darktable's `uk.po` and `pl.po` at the tag. darktable's tooltip names the folders it loads export ICC profiles from, which Apertrust does not read. So the promise is Apertrust's own, saying what the control does: "The colour profile the image is exported in." ([Gaps](#gaps), 2). Its Ukrainian and Polish are Apertrust's drafts, not darktable's strings.

## Invariants

The rows every first-pass table has (decision 22; [README.md](README.md), Rows every table has), but the first step, since every photo runs colorout, and order, since the module has one control; then the binding, what an edit cannot choose, layers and darktable's match. No control's unit is Apertrust's.

| Row | Control | Moved to | Measured | Must | Tolerance | Images |
| --- | --- | --- | --- | --- | --- | --- |
| O0 | Export profile at its default | sRGB, written or kept | `bind`; the plan's instances and cache keys; the frame | `bind` returns nothing; instances, keys and frames as with no edit | Exact | The small Bayer raw and the sweep through the export and the viewport; every corpus raw |
| O1 | Export profile, moved and reset | Each profile an edit can choose, sRGB today, then reset by `ResetControl` and by `ResetModule`; each profile held back, linear Rec709 and linear Rec2020, bound directly, as a held-back control's rows are (`tool-bindings`, Testing a module), then gone | The profile colorout renders; the frame on one pipeline updated as the session updates it | The profile chosen is the one rendered; once reset, or the held-back profile gone, the frame and its keys are no edit's; while sRGB is set too, and a held-back profile's frame is not sRGB's | Exact | The small Bayer raw and the sweep through the export and the viewport; the 90D through the viewport and the export |
| O2 | The binding | Each of the twelve profiles | `bind`'s settings | Each profile but sRGB writes colorout's `type` as the operation's choice of its name, and nothing else; sRGB writes nothing | Exact | CPU, a colour raw's context and an image that is not a raw |
| O3 | What an edit cannot choose | Each profile but sRGB; `--param colorout.type` and `colorout.intent` | The edit; the operation's `prepare` | Each command refused, the edit unchanged: the two linear profiles held back, which the operation prepares, and the nine others, which it refuses as not ported; the profile refused at operation level as owned, the intent as fixed | Exact | CPU |
| O4 | On a layer | | The definition | Background only: never offered on a layer, as colorout encodes the whole image for its output | Exact | CPU |
| O5 | The darktable match | None | The definition's list; the profile and intent an unedited photo, and each untouched edit, renders | darktable's export profiles in `dt_colorspaces_init`'s order with its preferences at their defaults; colorout at darktable's `$DEFAULT`s, sRGB and perceptual, which `apertrust-golden pipeline` holds to darktable-cli's sRGB export | Exact | CPU; the small raw and the sweep; every corpus raw |

O5 is how the module matches darktable. An untouched module binds nothing, so colorout runs its defaults, darktable's, and the whole-pipeline exit test compares that sRGB output with darktable-cli's 16-bit sRGB export, 27 of the corpus's 29 raws passing. The exit test renders each raw with its defaults and never reads an edit, so the module cannot move it. No golden case moves colorout's profile.

## Results

Measured on the Apple M4 (Metal) by `crates/modules/tests/output_colour_profile.rs` and the module's source. The small-image and CPU rows run on CI's software GPUs too; the corpus rows run with `APERTRUST_RAW_CORPUS`, `#[ignore]`d.

| Row | Result |
| --- | --- |
| O0 | Holds, exactly: `bind` sets nothing with Export profile written at sRGB or kept there, and the frames are byte-identical to no edit on the small raw and the sweep through the export and the viewport. On all 31 corpus raws the pipeline takes, the same instances and cache keys as no edit |
| O1 | Holds, exactly, on the small raw and the sweep through the export and the viewport, and on the 90D through the viewport and the export (below) |
| O2 | Holds for all twelve profiles on both contexts |
| O3 | Holds, exactly: eleven profiles refused as not ported, the edit unchanged; of them the operation prepares exactly linear Rec709 and linear Rec2020; `--param colorout.type` refused as owned and `colorout.intent` as fixed, as `apertrust-cli edit` reports them |
| O4 | Holds: the definition works on the background only, with no Auto |
| O5 | Holds: the list is darktable's twelve in its order; the small raw, the sweep and all 31 corpus raws render colorout at sRGB and perceptual, with no edit and with each untouched one |

O1 in detail: sRGB set renders as no edit, and so does each reset. Linear Rec709 and linear Rec2020, bound, render in their own profiles, not sRGB's, and once they are gone the pipeline renders no edit's bytes and keys again.

## In the panel

- **No tabs:** Export profile alone, a list drawn disabled at sRGB (web-safe), as darktable's page has only it with its preferences at their defaults (`output_colour_profile_draws_export_profile_at_srgb_disabled`). Disabled, it does not open, so the other profiles are not seen yet.
- **Once the control is available**, when the display converts from the edit's profile, its list opens: each profile an edit still cannot choose drawn disabled with "Coming in a later version", the nine colorout does not port among them.
- **While closed** the section shows its title and buttons; its presets and options are drawn disabled until something is behind them.

## Gaps

What darktable's page has or does that the panel does not yet:

1. **The linear profiles.** linear Rec709 and linear Rec2020 wait for the display path to convert colorout's output from the edit's profile (decision 26; `docs/gui.md`, Display colour), and the CLI's PNG for a tag of the profile it holds, not sRGB's. Then they leave the definition's `unported` list, the control becomes available, and O1 checks each through the edit, as it checks sRGB now.
2. **ICC files.** darktable lists the ICC profiles in its `color/out` folders after the built-in ones, chosen by `type` file and their `filename`, and its tooltip names those folders. The operation reads no ICC file and has no `filename` parameter (`docs/ports/colorout.md`, Parameters), and a definition's options are static. So the list holds the built-in profiles alone, and the promise says what the control does in place of darktable's tooltip. The files come with an ICC port, their options with a choice whose list follows what is installed.
3. **The profiles colorout does not port:** Adobe RGB (compatible), Rec709 RGB, the PQ and HLG profiles, Display P3 RGB, linear ProPhoto RGB and BRG (for testing), refused, and drawn disabled in the list once it opens.
4. **Softproofing and the gamut check,** darktable's darkroom toggles, which colorout reads from the pipe, not from its parameters: out of this module, with the display colour work.
5. **The module's tooltip,** darktable's "convert pipeline reference RGB to any display RGB using color profiles to remap RGB values": a definition implies no message for the module's own tooltip.

## Deliberate differences

- **sRGB only** ([Controls](#controls)): darktable lets every edit choose any export profile it lists. Here the two linear profiles the operation ports are held back until the display converts from the edit's profile, as decision 26 requires.
- **The viewport keeps the edit's profile** (decision 26, principle 2): darktable's darkroom renders colorout to the display's profile whatever the module holds. Apertrust's viewport renders the edit's, so the screen shows what the export holds. With sRGB the only choice, the two agree on an sRGB display.

## Design choices

- **The held-back profiles:** linear Rec709 and linear Rec2020 are refused as commands and the control is unavailable until the display path converts from the edit's profile, as decision 26 orders. Otherwise the viewport, the scopes and the CLI's sRGB-tagged PNG would show and save a linear profile's values as sRGB.
- **Allowing the linear profiles raises the module's version,** so an older build, which still holds them back, opens an edit holding `lin_rec2020` or `lin_rec709` for viewing only (decision 48). Without the raise, such a build would leave the value out with a warning as not ported (`sidecar`'s `an_option_not_ported_is_left_out_with_a_warning`), render sRGB, and drop the profile at its next save.
- **Control id:** `type`, the operation's parameter and darktable's params field, so `--set` and scripts name what darktable's history names.
- **Option ids:** the operation's choices by name, `apertrust_color::PROFILE_TYPES`, so the binding needs no table of its own.
- **The list:** darktable's twelve built-in export profiles in its order. Lab and linear XYZ are left out, as darktable's preference leaves them out (rule 12); ICC files are a gap.
- **The promise:** "The colour profile the image is exported in.", since darktable's tooltip names folders Apertrust does not read and would promise files.
- **Rows:** O0 to O5. No first step, since every photo runs colorout, and no order row, since there is one control. O1 binds the held-back profiles directly, so their binding is tested before they are choosable.
- **The panel's test:** in `crates/app/src/modules.rs`, beside every other module's.
- **Row ids:** O, for Output colour profile.
- **In `MODULES`:** after Demosaic, before White balance.
