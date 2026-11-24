# 0106. Presets and styles as .appreset and .apstyle files

## Context

Every tool's header has a presets button, drawn disabled until presets come ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)); Export's presets are designed ([0090](0090-export-as-a-photos-tab-tool.md)), Import's too ([0085](0085-import-in-folder-mode.md)); History's ⋯ holds Create a Style…, and the Edit menu Copy Edit, Apply Edit and Save Edit as Style…, all waiting for one model. This record settles that model: where presets and styles are kept, what a file holds, how a file a build cannot fully use is read, and how presets, styles and a copied edit are applied.

darktable at the reference release keeps presets and styles in `data.db`, its `presets`, `styles` and `style_items` tables, each module's parameters a C struct blob at its `op_version`, with `.dtpreset` and `.dtstyle` XML only as exports. Apertrust keeps no database as the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), already has an XMP format for a tool's values with versions, migration and a newer build's rules ([0048](0048-the-apxmp-format.md)), and its engine opens no file ([0043](0043-settings-and-apertrusts-folders.md)).

## Decision

**Presets and styles are files, never a database,** as edits are ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), [0047](0047-where-edits-live-and-what-the-sidecar-holds.md)). Two file types, both XMP, each one preset or one style, so each can be sent, shared, backed up and synced on its own. All planned, in two phases of their own ([Phases](#phases)).

| File | Holds | Where |
| --- | --- | --- |
| `.appreset` | One preset of one tool | `<user data>/presets/<tool>/<Name>.appreset` |
| `.apstyle` | One style: one or more tools | `<user data>/styles/<Name>.apstyle` |

`<tool>` is the tool's id (`exposure`, `temperature`, `export`), and `<Name>` the name made safe as a file name (below). The user data folder is [0043](0043-settings-and-apertrusts-folders.md)'s; `apertrust_base::folders::Folders` gains `presets(tool)` and `styles()` beside `workspaces()`, each a folder that may not exist yet.

### What a file holds

**The `.apxmp`'s own encoding,** so reading one goes through the edit's code for migration, clamping and newer builds, and the same values give the same bytes:

- **The root `rdf:Description`** holds `apertrust:preset="1"` or `apertrust:style="1"`, the file's kind and format version (never `apertrust:version`, so neither is ever taken for an edit); `apertrust:name`, the name as typed; and an optional `apertrust:description`, plain text. `x:xmptk` names the build that wrote it, as an edit's does.
- **`apertrust:tools`** is an `rdf:Seq` of tool entries exactly as a version 1 `.apxmp` writes them (`apertrust_library::sidecar::Entry::Tool`): `apertrust:tool`, `apertrust:toolVersion`, `apertrust:kept`, `apertrust:auto`, `apertrust:enabled` and the `control:<id>` values as text, in the `https://apertrust.org/ns/xmp/` and `…/control/` namespaces.
- **A processing tool's preset** holds exactly one entry. **A style** holds one or more.
- **A switched tool** ([0032](0032-tools-off-until-first-touched.md)) is written as the `.apxmp` writes it: on, `apertrust:enabled="true"` with its values off their defaults; off, an entry with no `apertrust:enabled` and no values, which turns the tool off when applied, as a darktable style can. `false` is never written; read, it parks the file as the `.apxmp`'s reading treats it, a newer build's value.
- **No `apertrust:operations`:** a parameter no tool owns ([0028](0028-tools-bound-to-operations.md)) is never in a preset or a style.
- **Every field is checked,** as version 2 of the `.apxmp` checks a step's entries (`sidecar::copies`' `entry_problems`, shared with `presets`), not as version 1's reader, which drops a field it does not know: an entry's field other than its `tool`, `toolVersion`, `enabled`, `kept`, `auto` and `control:` fields, an item that is no entry, a repeated or qualified field, in `apertrust:settings` a field other than `tool` and the `setting:` ones, and at the root any field other than the kind, `name`, `description`, `tools` or `settings`, of Apertrust's namespace or another, parks the file, since writing it back would drop what a newer build put there.
- **A utility's preset,** Export's and later Import's, holds no tool entries but `apertrust:settings`, a struct of `apertrust:tool`, the utility's id, and `setting:<key>="<value>"` fields, each a setting its page stores by its full key (`setting:export.format="jpeg"`), its value as text, as `--conf` takes it, in a new namespace, `https://apertrust.org/ns/xmp/setting/` (prefix `setting`). Saving writes every setting its page stores, but one no preset sets, Export's `export.ask_before_overwrite`.
- **A utility preset's settings follow the settings' rules** ([0043](0043-settings-and-apertrusts-folders.md)), as `settings.toml`'s do: a key the build does not know is warned of under `-d settings`, never applied, and kept, written back as read when the file is updated or renamed, so a newer build's setting survives; a value its kind cannot read is warned of and its setting takes its default. **Applying one** sets every setting its page stores: to the file's value, or to its default where the file lacks the key, as an older build's file does, so a preset gives the same settings whatever was set before, as a processing tool's preset sets its controls; a setting no preset sets, and one `--conf` holds for the run, are left as they are.

```xml
<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Apertrust 0.1.0">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:apertrust="https://apertrust.org/ns/xmp/"
    xmlns:control="https://apertrust.org/ns/xmp/control/"
    apertrust:preset="1"
    apertrust:name="Bright interiors">
   <apertrust:tools>
    <rdf:Seq>
     <rdf:li>
      <rdf:Description
        apertrust:tool="exposure"
        apertrust:toolVersion="3"
        apertrust:kept="exposure"
        control:exposure="1.2"/>
     </rdf:li>
    </rdf:Seq>
   </apertrust:tools>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
```

**Writing** goes through the `.apxmp`'s writer for the entries and its safe write, `apertrust_library::sidecar::write_xml` ([0053](0053-finding-folders-and-photos.md)), in one fixed layout, so a file already in that layout is written back as the same bytes; other valid XMP is rewritten in it.

**The code** is `apertrust_library::presets`, a module of the library outside its four layers ([0054](0054-four-layers-for-the-librarys-code.md)), which are the photo folders' and are reached only through `LibraryCommand`. Presets and styles are the user data folder's, which the engine never opens ([0043](0043-settings-and-apertrusts-folders.md)), so the clients call `presets` directly, as they read `settings.toml` and the workspaces themselves: the app and the CLI depend on `apertrust-library` for it, and nothing in the four layers calls it. It knows no tool and no value's meaning, as `sidecar` knows none:

- a file's kind, name, description, entries (`sidecar::Entry::Tool`), settings and format version as text, parsed and written over `xmp` with `sidecar`'s entry reading and writing and its field check, made shared within the crate, and `sidecar::write_xml`;
- a folder's files listed, sorted with `listing::browser_order`, a pure comparison of two names that touches no folder; written, renamed and deleted; a name made a file name;
- the built-in files embedded, and the folders from `apertrust_base::folders::Folders`, which the client passes in.

The engine turns a file's entries into tool values with the code that reads an edit's entries, a pure function the client calls; the engine never opens a preset or a style, which clients read and send as values.

### Names and file names

- **A name** is the text a person types, its spaces at both ends trimmed, never empty. It is unique within its tool for presets, and among styles, compared exactly, as darktable's are; it may not be a built-in preset's name as shown. A clash is refused in the dialog, never resolved silently.
- **The file name** is the name with `/ \ : * ? " < > |` and control characters replaced by `_`, its leading and trailing dots and spaces trimmed, and cut to 200 bytes at a character boundary; nothing left gives `_`, and a name whose part before its first dot is one of Windows' reserved device names (`CON`, `PRN`, `AUX`, `NUL`, `COM1` to `COM9`, `LPT1` to `LPT9`, in any case) gets `_` after that part. A file name already taken in the folder, compared ignoring case so that the folder survives a copy to any file system, gets " 2", " 3" and so on before the extension.
- **The name lives in the file,** so the file name is only a handle: "Web" and "web" are two presets on any file system, `web 2.appreset` holding the second. Renaming rewrites `apertrust:name` and renames the file.
- **Listed** by name in `apertrust_library::listing::browser_order`, after the built-ins, as darktable lists its write-protected presets first.

### Reading

| A file | Is |
| --- | --- |
| Of a format version above this build's | **Parked:** listed but disabled, saying why ("Made by a newer Apertrust"), never rewritten |
| With a field the check above finds (`presets`, from the text) | Parked, saying some of it can't be kept |
| A preset whose entry has a newer `toolVersion`, names a tool this build neither has nor has retired (`apertrust_tools::RETIRED`), or holds a value the `.apxmp`'s reading would make view-only | Parked, saying why |
| A style with such an entry | Listed and applied without it, its tooltip and the notice after applying naming what is left out; the file is never rewritten, so nothing is lost |
| A style's entry of a retired tool | Left out with a warning, as an edit's is; a preset of one is parked, having nothing left to apply |
| An entry of an older tool version | **Migrated in memory** by the tool's `ProcessingTool::migrate`, and clamped as an edit's values are; the file is not rewritten until the person updates it |
| Not XMP, not of its extension's kind, naming nothing, or a preset in a folder other than its tool's | **Ignored,** with a warning under `-d settings` |

**Who tells what:** `presets` tells a format version above its own and an unknown field from the text alone. What needs the tool definitions, a newer tool version, a tool unknown or retired, a value out of reach, is told by the engine's reading of one entry into tool values, which returns the values, migrated and clamped, or why the entry is not used (a newer tool, an unknown tool or a retired one), which parks a preset and leaves a style's entry out; the client calls it on each entry as it lists the files, so a parked preset is drawn disabled before anyone picks it.

### Built-in presets

- **Never copied into the user data folder:** each build applies its own, so none is ever stale.
- **Default settings,** first in every list, comes from the tool's definition: its controls' defaults for the photo, and off for a switched tool, as `Command::ResetTool` sets them; for a utility, every setting its page stores at its default but one no preset sets, and a setting `--conf` holds for the run left as it is.
- **Export's For sharing and Full size JPEG** ([0090](0090-export-as-a-photos-tab-tool.md)), Import's Original names and By capture date ([0085](0085-import-in-folder-mode.md)) and, in phase 2, darktable's presets of each ported tool are files in the repository's `data/presets/<tool>/<id>.appreset`, in the format above, embedded at build time by `apertrust-library`, the one crate that parses them ([0045](0045-a-top-level-data-folder.md)), so the app and the CLI both have them. Each shows the Fluent message `preset-<tool>-<id>` as its name, `apertrust:name` holding its British English name for a reader of the file.

### Applying

- **A preset** replaces its tool's values on the active photo: every control it holds takes its value and marks, every other goes back to its default for the photo, unkept, and a switched tool turns on or off as the entry says. One history step, source You, noted with the preset's name as a message's argument ([0078](0078-undo-redo-and-the-edits-history.md)), sent as a `Command::Step` holding a command that sets a tool's whole values (planned in the engine). Default settings sends `Command::ResetTool`.
- **A utility's preset** writes its settings through the settings store, as the utility's rows do; it makes no history step, since a utility edits no photo.
- **`apertrust-cli export --preset NAME`** takes a built-in by its id (`for-sharing`) first, then the person's preset whose name is exactly NAME, then a built-in by its British English name; none is an error listing the presets there are. Its settings apply, its size settings included, though the CLI otherwise never takes the size from `export.size` ([0056](0056-export-as-an-engine-job.md)); any of `--width`, `--height`, `--hq` and `--upscale` given, and every other flag, win over them, so `--preset for-sharing` means 2048 px.
- **A style** applies its tools as one change: a step of each tool, sharing a batch, of source Style, noted with the style's name (`Command::Step`, [0078](0078-undo-redo-and-the-edits-history.md)). **Append** sets only the style's tools; **Replace** first resets the whole edit, as `Command::Reset` does, then sets them, one Undo for all. The person chooses Append or Replace when applying, Append by default; the file does not store it, as in darktable.
- **Copy Edit** keeps an unnamed style in the app's memory for the run, never on disk: every tool the active photo's edit holds (values off their defaults, a mark, or a switched tool on), but those unsafe to copy (below). **Apply Edit** applies it as Append, source Style, noted with the photo it was copied from.
- **On every selected photo,** Apply Edit and a style apply as one batch, one Undo for all, as [history.md](../history.md) plans a batch on several photos, built after the active photo's; a preset applies to the active photo alone, as darktable's do.
- **No hover preview** in phase 1: a preset or style applies on a click. Phase 2 adds the preview the quality bar asks for ([principles.md](../principles.md), Immediate).

### Copying between photos, and a new style

- **`ProcessingDefinition::copy_safe`,** true by default, is false for the tools darktable flags `IOP_FLAGS_UNSAFE_COPY` at the reference release, settings that belong to one photo: `flip` (Orientation), `rawprepare` (Raw black/white point) and `temperature` (White balance) now, and `lens` (Lens correction) when it comes. darktable's other four flagged modules, gamma, mask manager and rotate pixels, which are hidden, and scale pixels, are no tools of the first pass; any that becomes one is false too.
- **Copy Edit leaves those tools out,** as darktable's own copy does (`dt_history_module_skip_copy`, which skips deprecated, unsafe-to-copy and hidden modules). Choosing them by hand comes with Copy or Apply Parts… in phase 2.
- **`ProcessingDefinition::in_styles`,** false by default, is true for the tools darktable flags `IOP_FLAGS_INCLUDE_IN_STYLES` at the reference release, those that make a look rather than correct one photo. Of the built tools: `agx` (AgX), `channelmixerrgb` (Colour calibration), `colorbalancergb` (Colour balance RGB), `toneequal` (Tone equalizer), `hazeremoval` (Haze removal), `primaries` (RGB primaries), `grain` (Grain), `soften` (Soften) and `vignette` (Vignetting). The rest, Exposure, White balance, Orientation, Raw black/white point, Highlight reconstruction, Demosaic, Tone curve and the input and output colour profiles, carry none, as their modules carry none in darktable. Each later tool takes its module's flag: Local contrast, Bloom, Sigmoid, Filmic RGB, the colour equalizer, Chromatic aberrations and Diffuse or sharpen carry it; RGB curve, RGB levels, Sharpen, Denoise (profiled), Lens correction, Crop and Rotate and perspective do not.
- **Create a Style… lists every tool the edit holds, checked where its `in_styles` is true,** as darktable's create-style dialog checks only the modules flagged `IOP_FLAGS_INCLUDE_IN_STYLES` (`src/gui/styles_dialog.c`, 810 to 820). The unsafe-to-copy tools carry no `in_styles` in darktable either, so they start unchecked too. The person can check or uncheck any tool before creating the style.
- **Copy Edit is unchanged by `in_styles`:** it takes every tool the edit holds but those whose `copy_safe` is false, as above.
- **A tool's Copy Settings and Paste Settings,** in its ⋯ ([0091](0091-context-menus-places-and-tools-items-as-actions.md)), copy one tool: its values as an unnamed preset in memory, pasted onto the same tool of the active photo as one step. `copy_safe` does not apply, since the person names the tool. Phase 2.

### Phases

Presets and styles' own two phases, apart from the project's:

- **Phase 1:** this format and its module; tool presets in every processing tool's header; Export's presets with its two built in and `apertrust-cli export --preset NAME`; creating a style; the Styles tool applying one to the active photo; Copy Edit and Apply Edit on the active photo; then Apply Edit and styles on every selected photo, as one batch.
- **Phase 2:** the hover preview; darktable's built-in presets; Copy or Apply Parts…; a tool's Copy Settings and Paste Settings; Favourites, editing and duplicating a style; a preset's and a style's actions, with their ids ([0098](0098-actions-and-shortcuts-after-darktable.md)); styles on export and import; `.appreset` and `.apstyle` opened from outside; importing `.dtstyle` and `.dtpreset`; presets applied automatically; `--preset` and `--style` for processing tools on the CLI. Blending and several instances in presets and styles wait for layers (Phase 4).

## Options considered

- **A database, as darktable's `data.db`:** a second truth beside the files, which a file manager cannot carry and a sync folder breaks ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)).
- **TOML presets beside XMP styles,** a `presets/export.toml` of a table per preset: two encodings of the same values, and a second reader to keep with the first through every tool version.
- **One file holding all of a tool's presets:** names need no escaping, but a preset cannot be sent, shared or synced one at a time, and two computers changing different presets of one tool conflict.
- **darktable's `.dtpreset` and `.dtstyle` as the format:** C struct blobs tied to each module's `op_version`, unreadable without darktable's struct layouts, and nothing for a utility's settings. Importing them is phase 2.
- **The file name as the name:** a name could not hold `/` or `:`, and "Web" and "web" would be one file on the Mac's and Windows' file systems.
- **`apertrust:enabled="false"` for a switched tool off:** a value the `.apxmp` never writes and its reading takes for a newer build's; an entry with no values already says off.
- **The copy flag on `ToolDefinition`:** every utility would carry a field with no meaning for it; the processing tool's own declaration holds it, and so does `in_styles`.
- **Create a Style… checking every tool the edit holds but the unsafe-to-copy ones,** one flag, `copy_safe`, for Copy Edit and a new style alike: it would carry each photo's exposure and white balance into a look meant for other photos, and it is not darktable's behaviour.
- **Built-ins embedded by the app:** the CLI's `--preset for-sharing` would not have them.
- **`presets` as layer 1 of the four, or through `LibraryCommand`:** layer 1 lists no folders and clients see only layer 4, and a command would have the engine open the user data folder, which it never does; the four layers are a photo folder's.
- **Presets in `apertrust-base`,** beside the settings and workspaces: base would need the XMP layer and the entry codec, which are the library's, and the library already depends on base.
- **A utility preset leaving a setting it lacks as it is:** the result would depend on what was set before, unlike a processing tool's preset.
- **Unknown setting keys dropped:** a newer build's setting would be lost the first time an older build updated the preset.
- **A style parked whole for one newer entry:** the file is never rewritten, so applying the rest loses nothing, and the notice says what was left out.

## Consequences

- `apertrust-library` gains its `presets` module, outside the four layers, and `SETTING_NAMESPACE`; the app gains a dependency on it, which the CLI has already; [0054](0054-four-layers-for-the-librarys-code.md) and [library.md](../library.md) name the module beside the layers; with fixtures written back byte for byte; `apertrust-base`'s `Folders` gains `presets(tool)` and `styles()`; `data/presets/` its first files, with their rows in `data/README.md`.
- The engine gains a command that sets a tool's whole values as one change, allowed inside `Step`, and the reading of an entry into tool values, parked or migrated, through the edit's code.
- `apertrust-tools` gains `ProcessingDefinition::copy_safe` and `ProcessingDefinition::in_styles`, each tool's value from its darktable module's flags at the reference release, held by a test.
- Export's presets are `presets/export/<Name>.appreset` and Import's `presets/import/<Name>.appreset`, with no TOML presets file; `apertrust-export` reads and writes no presets.
- The docs: [gui.md](../gui.md) (The Export tool, The Import sheet, One tool model), [library.md](../library.md) (what is stored where), [settings.md](../settings.md) (the user data folder), [engine.md](../engine.md) (Export jobs), [crates.md](../crates.md) and the [roadmap](../roadmap.md).
