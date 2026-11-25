# 0106. Presets and styles as .appreset and .apstyle files

## Context

Every tool's header has a presets button, drawn disabled until presets come ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)); Export's presets are designed ([0090](0090-export-as-a-photos-tab-tool.md)), Import's too ([0085](0085-import-in-folder-mode.md)); History's ⋯ holds Create a Style…, and the Edit menu Copy Edit, Apply Edit and Save Edit as Style…, all waiting for one model. This record settles that model: where presets and styles are kept, what a file holds, how a file a build cannot fully use is read, how presets, styles and a copied edit are applied, and what a photo remembers of the styles applied to it, so that one can be cleared later.

darktable at the reference release keeps presets and styles in `data.db`, its `presets`, `styles` and `style_items` tables, each module's parameters a C struct blob at its `op_version`, with `.dtpreset` and `.dtstyle` XML only as exports. It remembers a style applied to a photo only as a tag, `darktable|style|<name>` (`src/common/styles.c:965-972`), with no order and nothing to take one off alone; every such tag goes whenever the photo's history is deleted (`src/common/history.c:137-138`), as by Discard History, the lighttable's Undo, which restores a history that way (`src/common/history_snapshot.c:169`), a style applied in overwrite mode (`src/control/jobs/control_jobs.c:1801`, `src/common/styles.c:830`), Compress or Truncate at end 0 (`src/common/history.c:1214`, `:1359`) and an overwriting paste (`:1949`, `:1958`). Apertrust keeps no database as the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), already has an XMP format for a tool's values with versions, migration and a newer build's rules ([0048](0048-the-apxmp-format.md)), and its engine opens no file ([0043](0043-settings-and-apertrusts-folders.md)).

## Decision

**Presets and styles are files, never a database,** as edits are ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), [0047](0047-where-edits-live-and-what-the-sidecar-holds.md)). Two file types, both XMP, each one preset or one style, so each can be sent, shared, backed up and synced on its own. In two phases of their own ([Phases](#phases)): the files, processing tools' presets and Export's are built; Import's presets and styles are planned.

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

- **A name** is the text a person types, its spaces at both ends trimmed, never empty. It is unique within its tool for presets, and among styles, compared exactly, as darktable's are; it may not be a built-in preset's or built-in style's name as shown. A clash is refused in the dialog, never resolved silently.
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

### Built-in styles

- **Eight styles ship with Apertrust,** each one tool's preset of darktable's at the reference release, its values those darktable's `init_presets` sets, written as the tool's controls by the tool's own tables, each held to its preset control by control by a test, so no look is invented:

  | Built-in style | id | darktable's preset |
  | --- | --- | --- |
  | Black and white › Black and white | `bw-luminance` | Colour calibration › monochrome › luminance-based |
  | Black and white › HP5+ | `bw-hp5` | Colour calibration › monochrome › ILFORD HP5+ |
  | Black and white › Acros 100 | `bw-acros-100` | Colour calibration › monochrome › Fuji Acros 100 |
  | Natural colour | `natural-colour` | Colour balance RGB › basic colorfulness › natural skin |
  | Vibrant colour | `vibrant-colour` | Colour balance RGB › basic colorfulness › vibrant colors |
  | Punchy | `punchy` | AgX › blender-like › punchy |
  | Open shadows and highlights | `open-shadows-highlights` | Tone equalizer › compress shadows/highlights › EIGF › medium |
  | Lomo | `lomo` | Vignetting › lomo |

- **Files in the repository's `data/styles/<id>.apstyle`,** in the format above, embedded by `apertrust-library` (`presets::builtin_styles`) as the built-in presets are, and never copied into the user data folder. Each shows the Fluent message `style-builtin-<id>` as its name, a `|` in it making its folders, as a person's style's does, and `apertrust:name` holds its British English name.
- **Read-only:** a built-in style applies, clears and shows as applied as any style does. Its menu holds **Copy to Custom Styles…** in place of Rename Style… and Delete Style…: the name dialog, its name at first, writes an `.apstyle` holding the built-in's entries into `<user data>/styles/`, under a name no style holds, built in or the person's.
- **Applied,** it is listed under its name as shown, its `file` `builtin:<id>` (`builtin:lomo`): no file name holds a `:` (above), so it never names a file of the person's. The Styles tool knows a built-in applied by that file, in whatever language it was applied in, and a person's style by its name only where the item's file is no built-in's, so a person's style named as a built-in is never taken for it; a style is applied from the tree by its file. **Clear** sends the name the built-in was applied under, and **applying it again** while it is on the list sends that name too, so the engine, which matches by name, moves it to the top rather than listing it twice. Planned with styles on several photos, whose lists the client does not know ([0109](0109-a-style-on-every-selected-photo-one-batch-one-undo.md)): the engine matches by file where either the item's or the applied style's file is a `builtin:<id>`, the same style only where both files are the same, and by name only where neither is; a built-in matched takes the name sent.
- **The Styles tool** draws two root folders, Custom Styles first, the person's own, then Built-in Styles ([gui.md](../gui.md#the-styles-tool)).

### Applying

- **A preset** replaces its tool's values on the active photo: every control it holds takes its value and marks, every other goes back to its default for the photo, unkept, and a switched tool turns on or off as the entry says. One history step, source You, noted with the preset's name as a message's argument ([0078](0078-undo-redo-and-the-edits-history.md)), sent as a `Command::Step` holding a command that sets a tool's whole values (planned in the engine). Default settings sends `Command::ResetTool`.
- **A utility's preset** writes its settings through the settings store, as the utility's rows do; it makes no history step, since a utility edits no photo.
- **`apertrust-cli export --preset NAME`** takes a built-in by its id (`for-sharing`) first, then the person's preset whose name is exactly NAME, then a built-in by its British English name; none is an error listing the presets there are. Its settings apply, its size settings included, though the CLI otherwise never takes the size from `export.size` ([0056](0056-export-as-an-engine-job.md)); any of `--width`, `--height`, `--hq` and `--upscale` given, and every other flag, win over them, so `--preset for-sharing` means 2048 px.
- **A style** applies its tools as one change: a step of each tool, sharing a batch, of source Style, noted with the style's name, `history-style` (`Command::ApplyStyle`, [below](#the-styles-applied-to-a-photo); [0078](0078-undo-redo-and-the-edits-history.md)). **Append** sets only the style's tools; **Replace** first resets the whole edit, as `Command::Reset` does, then sets them, one Undo for all; it always records its Reset step, an empty one where the reset moved no tool, so the log says the edit was reset. The person chooses Append or Replace when applying, Append by default; the file does not store it, as in darktable.
- **Copy Edit,** planned and designed apart from the phases below (#372), keeps an unnamed style in the app's memory for the run, never on disk: every tool the active photo's edit holds (values off their defaults, a mark, or a switched tool on), but those unsafe to copy (below). **Apply Edit** applies it as Append, source Style, noted with the photo it was copied from.
- **On every selected photo,** a style applies as one batch, one Undo for all, as [0109](0109-a-style-on-every-selected-photo-one-batch-one-undo.md) decides (planned, #373): one `ApplyStyle` naming the photos, the active one changed at once and the others by a job with no decode or GPU, a view-only photo skipped and named; Clear stays the active photo's. So, once designed, does Apply Edit (#372); a preset applies to the active photo alone, as darktable's do.
- **No hover preview** in phase 1: a preset or style applies on a click. Phase 2 adds the preview the quality bar asks for ([principles.md](../principles.md), Immediate).

### Copying between photos, and a new style

- **`ProcessingDefinition::copy_safe`,** true by default, is false for the tools darktable flags `IOP_FLAGS_UNSAFE_COPY` at the reference release, settings that belong to one photo: `flip` (Orientation), `rawprepare` (Raw black/white point) and `temperature` (White balance) now, and `lens` (Lens correction) when it comes. darktable's other four flagged modules, gamma, mask manager and rotate pixels, which are hidden, and scale pixels, are no tools of the first pass; any that becomes one is false too.
- **Copy Edit leaves those tools out,** as darktable's own copy does (`dt_history_module_skip_copy`, which skips deprecated, unsafe-to-copy and hidden modules). Choosing them by hand comes with Copy or Apply Parts… in phase 2.
- **`ProcessingDefinition::in_styles`,** false by default, is true for the tools darktable flags `IOP_FLAGS_INCLUDE_IN_STYLES` at the reference release, those that make a look rather than correct one photo. Of the built tools: `agx` (AgX), `channelmixerrgb` (Colour calibration), `colorbalancergb` (Colour balance RGB), `toneequal` (Tone equalizer), `hazeremoval` (Haze removal), `primaries` (RGB primaries), `grain` (Grain), `soften` (Soften) and `vignette` (Vignetting). The rest, Exposure, White balance, Orientation, Raw black/white point, Highlight reconstruction, Demosaic, Tone curve and the input and output colour profiles, carry none, as their modules carry none in darktable. Each later tool takes its module's flag: Local contrast, Bloom, Sigmoid, Filmic RGB, the colour equalizer, Chromatic aberrations and Diffuse or sharpen carry it; RGB curve, RGB levels, Sharpen, Denoise (profiled), Lens correction, Crop and Rotate and perspective do not.
- **Create a Style… lists every tool the edit holds, checked where its `in_styles` is true,** as darktable's create-style dialog checks only the modules flagged `IOP_FLAGS_INCLUDE_IN_STYLES` (`src/gui/styles_dialog.c`, 810 to 820). The unsafe-to-copy tools carry no `in_styles` in darktable either, so they start unchecked too. The person can check or uncheck any tool before creating the style.
- **Copy Edit is unchanged by `in_styles`:** it takes every tool the edit holds but those whose `copy_safe` is false, as above.
- **A tool's Copy Settings and Paste Settings,** in its ⋯ ([0091](0091-context-menus-places-and-tools-items-as-actions.md)), copy one tool: its values as an unnamed preset in memory, pasted onto the same tool of the active photo as one step. `copy_safe` does not apply, since the person names the tool. Phase 2.

### The styles applied to a photo

Built: the list, its file format, the engine's commands and the Styles tool. A photo's edit remembers the styles applied to it, so that the Styles tool can list them and **Clear** can take one off later without undoing what came after it.

- **The list,** newest first: applying a style puts it at the top, and applying one already on it moves it there. Each item holds the style's name as applied, its file name in `<user data>/styles/` then, or `builtin:<id>` for a built-in style, and, for each tool it set, that tool's whole stored state before it and after it (values, marks, on or off), as entries. A style is on the list by its name, unique among styles, so one renamed or deleted since stays under the name it was applied with: Clear needs nothing from its file. A built-in style, whose name as shown follows the language, is recognised by its `builtin:<id>` file instead, and Clear and applying it again send the name it was applied under (above). A style renamed away and a new one created under its old name is taken for the one applied, as a person reading the list would take it; matching by name and file name as well would change nothing there, since the new file takes the old file name the rename freed.
- **Applied again,** a style's item keeps, for each tool that still holds what the earlier application set, the earlier state before it, so Clear goes back to before the style first set that tool; takes, for every other tool it sets, the state just before this application; and keeps the earlier item's tools this application does not set as they were.
- **Clear** puts back each tool the style set whose stored state is still the item's after state, compared as entries, to its before state, and takes the style off the list. A tool changed since keeps its value. Two styles that set a tool to the same state both count it as theirs: clearing the earlier puts the tool back to before it, though the later is still listed. After a **Replace**, each tool's before state is its reset state, so Clear leaves those tools at their defaults, as the Replace began. It is one change, a step of each tool it puts back, sharing a batch where there are several, source Style, noted `history-clear-style` with the style's name; one Undo takes it back, the list included.
- **A change that moves the list and no tool,** a Clear whose tools have all changed since or a style applied again whose tools already hold its values, is one step holding the list alone, as is the step Compress writes the list on (below), `Change::None`, its History line reading its note. A change that moves neither makes no step.
- **The list lives in the log** ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md), [0078](0078-undo-redo-and-the-edits-history.md)), as a tool's state does: the step that changes it holds the whole list after it (`Step::styles`), the last step of a style's change or of a Clear, or Reset's step; the edit's list is the one at the last step up to the end that holds any list, else empty. So Undo and Redo, `GoTo`, Undo's fallback after reopening and Truncate move it with the tools and need nothing of their own: Undo of a style's change puts the list back as it was before it, taking a newly applied style off and a re-applied one back to its earlier place and before state, Redo puts it back, and going to a step inside its batch shows the list before it. **Reset's step,** Replace's too, which a Replace always records, holds the empty list only where the list was not empty, so Reset and Replace empty it and a photo Reset with no style applied holds no `apertrust:styles`; Discard History leaves no step, so no list. **Compress** takes the list off every step it keeps and, where the edit's list is not empty, writes it on a step of its own after the last step it keeps up to the end, the end then on it: a list-only step, `Change::None`, source Style, noted `history-styles-applied` ("Styles applied", with no argument; the Styles tool shows which), with a new id and Compress's clock. Keeping the step that held the list instead would keep its tool's entry over a later one Compress drops, a reset among them, and change what the photo shows. A log whose end is already on such a step holding the edit's list, as one compressed before, keeps it as it is, so Compress again changes nothing. So a style cleared before Compress stays cleared.
- **A newer file taken in** ([0052](0052-comparing-two-copies-of-a-sidecar.md)): the steps the session lays have their lists rebuilt by replaying the session's changes of the list, in order, onto the list at the newer log's end. **Every Reset step laid empties it,** whether or not that step stores a list, so the other side's styles never outlive the session's Reset; a step that stores a list replays what it differs by from the list before it in the session's log: a style added or moved to the top is put at the top, any item of its name taken out first, and a style cleared is taken out where the list holds it. Each laid step then holds the list rebuilt, a Reset the empty list where the rebuilt one before it was not empty. **A style replayed** stays at the top, its after states the session's, and each tool's before state is what that tool holds in the joined log just before the step that first set it for this item, an earlier application's for a tool a later one did not set, not what the session recorded: the session applies A, setting X from 0 to 1, while the newer file set X to 2, and the replayed item's before for X is 2; a style already on the rebuilt list is applied again there by the rule above. So the styles the other side applied stay listed beside the session's, as decision 52 keeps each side's change tool by tool.
- **Only named styles** are listed: Apply Edit, a copied edit with no name or file, and a preset add nothing to it.

**Commands and values.** `ApplyStyle` and `ClearStyle` are commands on the edit, each ending a gesture under way, neither allowed in a `Step` (`NotInStep`), both refused on a photo open for viewing only or not yet live:

- **`ApplyStyle { name, file, mode, tools }`:** the client reads the `.apstyle`, turns each entry into tool values with `sidecar::tool_values`, leaving a parked one out, and sends the name, the file name, `StyleMode::Append` or `Replace`, and the tools' values. The session applies it as `Step { source: Style, note, commands }` would, `Reset` first for Replace, then a `SetTool` for each tool, taking each tool's before state from its own edit, and updates the list in the same change.
- **`ClearStyle { name }`:** Clear, as above; refused, `UnknownStyle`, where the list holds no style of that name.
- **`Values::styles`:** the list, newest first, each its name, file name and the ids of the tools it set, which the Styles tool draws.

**In the `.apxmp`,** version 2, a step that changes the list holds `apertrust:styles` after its `apertrust:entries`: an `rdf:Seq` of the list after it, newest first, closed at once (`<rdf:Seq/>`) where the step emptied it. Each item is a struct of `apertrust:name`, `apertrust:file`, a file name and never a path, since the folder differs from one computer to another, or `builtin:<id>` for a built-in style, and `apertrust:before` and `apertrust:after`, each an `rdf:Seq` of tool entries as version 1 writes them, the same tools in the same order, an entry with no values a tool's reset state. A step that leaves the list as it was holds none, so an edit no style was ever applied to holds none. A step holding the list alone, `Change::None`, writes `apertrust:entries` as an empty `rdf:Seq` (`<rdf:Seq/>`), as every step writes its entries, then its `apertrust:styles`; a reader tells it from a Reset by its entries being empty, its note not `history-reset` and its `apertrust:styles`. The engine's reading of a step (`sidecar::typed_step`) takes a step of no entry and no list as an empty Reset, and one of no entry holding a list, its note not `history-reset`, as `Change::None`; and a step whose entries are all left out as it is read, a retired tool's, stays as `Change::None` where it holds a list, rather than being left out:

```xml
<apertrust:styles>
 <rdf:Seq>
  <rdf:li>
   <rdf:Description
     apertrust:name="Warm film"
     apertrust:file="Warm film.apstyle">
    <apertrust:before>
     <rdf:Seq>
      <rdf:li>
       <rdf:Description
         apertrust:tool="grain"
         apertrust:toolVersion="2"/>
      </rdf:li>
     </rdf:Seq>
    </apertrust:before>
    <apertrust:after>
     <rdf:Seq>
      <rdf:li>
       <rdf:Description
         apertrust:tool="grain"
         apertrust:toolVersion="2"
         apertrust:enabled="true"
         control:strength="40"/>
      </rdf:li>
     </rdf:Seq>
    </apertrust:after>
   </rdf:Description>
  </rdf:li>
 </rdf:Seq>
</apertrust:styles>
```

- **No version rise:** the file's version rises for removals and changes, never for additions ([0048](0048-the-apxmp-format.md)), so it stays 2.
- **An older version 2 build** reads `apertrust:styles` as a field of a step it does not write, `Problem::UnknownField`: the photo opens there for viewing only, its caption saying some of its history can't be kept, and nothing is written over it, unless the person chooses Discard History…, which warns first and starts the history afresh without it, as for any field that build cannot keep. Every edit no style was applied to, and every one whose list Compress or Discard History has left with no step holding it, reads in every version 2 build.
- **A build that has it** reads each before and after entry by decision 48's rules, as every step's entries: an older tool version migrated, a newer one or an unknown tool making the photo view-only. An item that is no struct, one lacking its name or file, a before and an after that differ in their tools, or a name twice in one list is a `sidecar::Problem`, so the photo opens for viewing only rather than lose it.

### Phases

Presets and styles' own two phases, apart from the project's:

- **Phase 1:** this format and its module; tool presets in every processing tool's header; Export's presets with its two built in and `apertrust-cli export --preset NAME`; creating a style; the Styles tool applying one to the active photo, with the styles applied to it and Clear; the built-in styles, with the Styles tool's Custom Styles and Built-in Styles; then styles on every selected photo, as one batch (#373, [0109](0109-a-style-on-every-selected-photo-one-batch-one-undo.md)).
- **Apart from both:** Copy Edit and Apply Edit, on the active photo and on every selected photo, are designed separately (#372); what is decided of them here stands, `copy_safe` among it, built.
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
- **darktable's tag, `darktable|style|<name>`:** it has no order, nothing takes one off alone, anything that deletes the history, Undo of one style in the lighttable among them, takes every one off, and it holds nothing Clear could put back.
- **The list on the virtual copy, beside the log:** a second truth beside the log, which [0047](0047-where-edits-live-and-what-the-sidecar-holds.md) rules out; Undo's fallback after reopening, `GoTo` and Truncate would each need to move it as well.
- **The list worked out from the steps' notes:** Compress drops a style's steps, and their note with them, while their tools still hold its values; and a tool's before, which the log derives, becomes its reset state after Compress, so Clear would reset what the person had set before the style.
- **Names alone, without the tools' states:** Clear could only reset the style's tools to their defaults, losing what they held before it, or read the steps before it, which Compress drops.
- **Before states alone, comparing with the style's file:** the file may be renamed, deleted or changed since; the after states are what the style set on this photo, migrated and clamped.
- **Only the change on each step, a style added or cleared, replayed:** the fold would replay the log where it looks up each tool's last entry, and Compress, which drops steps, would break the replay.
- **Clear as a `Step` of `SetTool`s the client builds from `Values::styles`:** a client's mirror can be a batch behind the session, during a gesture or after a newer file is taken in, so it could put back a tool changed since; a `SetTool` cannot take the style off the list, so a command is needed anyway; and every client would repeat the comparison.
- **A style applied as a `Step` with a command naming it among the `SetTool`s:** the before states and the list's rules would depend on where that command sits; one command keeps them in the session, in one place.
- **A style applied again taking every before from just before it:** where the photo still held the style's values, Clear would then change nothing yet take the style off.
- **A style parked whole for one newer entry:** the file is never rewritten, so applying the rest loses nothing, and the notice says what was left out.

## Consequences

- `apertrust-library` gains its `presets` module, outside the four layers, and `SETTING_NAMESPACE`; the app gains a dependency on it, which the CLI has already; [0054](0054-four-layers-for-the-librarys-code.md) and [library.md](../library.md) name the module beside the layers; with fixtures written back byte for byte; `apertrust-base`'s `Folders` gains `presets(tool)` and `styles()`; `data/presets/` its first files and `data/styles/` the built-in styles, with their rows in `data/README.md`.
- The engine gains a command that sets a tool's whole values as one change, allowed inside `Step`, and the reading of an entry into tool values, parked or migrated, through the edit's code.
- Built, for the styles applied: the engine gains `ApplyStyle` with `StyleMode`, `ClearStyle` with its refusal `UnknownStyle`, `Step::styles` and `Change::None` in its `history`, and `Values::styles`, and folds, compresses, writes and reads the list with the log; `apertrust_library::sidecar` reads and writes `apertrust:styles` in a step, with a fixture ExifTool and exiv2 read whole; the messages `history-style` and `history-clear-style` in every locale.
- `apertrust-tools` gains `ProcessingDefinition::copy_safe` and `ProcessingDefinition::in_styles`, each tool's value from its darktable module's flags at the reference release, held by a test.
- Export's presets are `presets/export/<Name>.appreset` and Import's `presets/import/<Name>.appreset`, with no TOML presets file; `apertrust-export` reads and writes no presets.
- The docs: [gui.md](../gui.md) (The Export tool, The Import sheet, One tool model), [history.md](../history.md) (a step's fields, Compress, the commands, the sidecar), [session-protocol.md](../session-protocol.md) and [events.md](../events.md) (the commands and `Values`), [library.md](../library.md) (what is stored where, version 2), [settings.md](../settings.md) (the user data folder), [engine.md](../engine.md) (Export jobs), [crates.md](../crates.md) and the [roadmap](../roadmap.md).
