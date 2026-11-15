# 0084. Export as a sheet of export tools

## Context

Export holds darktable's export module row for row ([0056](0056-export-as-an-engine-job.md)). It is built as a native modal dialog, a sheet on the Mac ([0041](0041-dialogs-as-native-modal-windows.md)): a Presets column and a settings column with Boxed choices, acting on the active photo. Its settings are darktable's export settings, which darktable keeps in darktablerc and Apertrust in `settings.toml`, never in an edit: the same kind as a tool's own settings ([0029](0029-a-tool-is-darktables-page.md), [0043](0043-settings-and-apertrusts-folders.md)). darktable draws its export module in the lighttable's side panel, acting on the selected images, with its processing modules' widgets, parted by the section labels storage options, format options and global options, with metadata in a dialog of its own and multi-preset export a collapsible section, and ships no export presets. The questions are which photos Export shows and acts on, which tools its settings make, what the selection is after an export, and which presets it ships.

## Decision

**Export… opens the Export sheet** ([0086](0086-import-and-export-as-one-sheet-pattern.md)): the photos it was opened with, checked, in a grid beside a column of five export tools. Planned: until Export is on the sheet, the dialog keeps working as built ([gui.md](../gui.md#as-built)).

### The grid

- **It shows the photos Export was opened with:** the window's selection, or the active photo when nothing else is selected, all checked, in the grid panel's sort. One photo is shown alone, large and fitted, as a last look before export, without changing the kept Zoom.
- **Show all N in folder,** a toggle in the bar, off at each opening, brings in the photos the grid panel shows (the open folder; an album's from Phase 5), the set still checked. Turned off, the grid shows the set again: the photos opened with and any checked meanwhile, an unchecked one dimmed until the sheet closes, so an uncheck is undone with one click.
- **With nothing selected and no active photo,** the sheet opens on the folder with nothing checked. Export… is enabled while the grid panel shows a photo.
- **The bar** holds Select all, Select none and Select edited (the photos with an edit, the pencil's rule); there is no Checked only, since the set is the view.
- **The caption** has two lines: the grid panel's rating line, read-only, then the check box and the file name.
- **The marks,** on checked cells only, from the session's preview: the grid panel's own (the pencil, the lock, the warning sign), then what On conflict does with a file already there: `copy-plus` for a new name beside it, `replace` in `warning` for a replace, `ban` for a skip, each tooltip naming the files.
- **Before Phase 3's selection,** the active photo alone is checked, and the sheet already lets the person check more.

### The tools

| Tool | darktable's | Rows | At first |
| --- | --- | --- | --- |
| **Storage** | storage options | Target storage; the storage's rows: File on disk's Path with its file-name note and any problem, On conflict and its note | Open |
| **Format** | format options | File format; the format's rows, JPEG's Quality and Chroma subsampling by default; until metadata, the note "This version writes no EXIF, location or copyright into exported files." | Open |
| **Global options** | global options | Set size; the size row or Scale, with their notes; Allow upscaling, High quality resampling and Store masks; Profile, Intent, Style, and Mode while a style is chosen | Open |
| **Metadata** | the ☰'s preferences, "edit metadata exportation" | General settings: its toggles; Per metadata settings: the Redefined tag / Formula table | Closed; disabled until metadata, its tooltip saying exported files carry none yet |
| **Multi-preset export** | multi-preset export | A toggle per preset of the person's own; Start batch export | Closed; disabled until presets |

- **The titles** are darktable's labels in sentence case and British English, "options" dropped where what is left is a noun naming what the tool sets: Storage and Format. "Global" alone names nothing, so Global options keeps darktable's label whole.
- **Each setting a tool draws has one owning tool** among the five, as each parameter has one ([0004](0004-tools-for-people-operations-for-algorithms.md)). The sheet's own settings, its column's width and order, Keep open and the grid's Zoom, are the app's, and `export.ask_before_overwrite` is the overwrite alert's, drawn by no tool.
- **Allow upscaling, High quality resampling and Store masks are toggles,** where darktable has comboboxes of No and Yes: a deliberate difference from darktable. Their settings are booleans already.
- **Inside Metadata,** darktable's General settings and Per metadata settings are group labels.
- **The rows' dependencies hold in any order,** being by value: the format list follows the storage, a storage's recommended size sets Global options' size, Copy disables Global options.

### Presets

- **Whole-export presets** in the sheet bar's presets row ([0086](0086-import-and-export-as-one-sheet-pattern.md)), kept in `presets/export.toml`: Default settings first; then two built in, **For sharing** (JPEG, 2048 × 2048 px, sRGB) and **Full size JPEG** (JPEG, no size limit, sRGB), the rest of each at darktable's defaults, a deliberate difference from darktable, which ships none; then the person's own, which Multi-preset export runs.

### Exporting

- **The footer:** "12 photos · JPEG · 2048 px → ~/Pictures/2026-10 Wedding/apertrust_exported"; its second line "Exports run in the background; you can keep working.", or in `warning` "Replaces 3 files already there", which says "at least" and how many photos' names are known only as they are exported where some names are not exact, or the reason Export is disabled.
- **Export** sends `Command::Export` with the checked photos in the grid's order, after the overwrite alert where it asks, the main window's.
- **When Export is pressed, the photos sent to export become the window's selection,** so Export Again repeats them, all of them after a failure or a cancel too, where the job's Try again exports only those not written; Close leaves the selection as it was. Before Phase 3's selection, the window holds only the active photo, so Export Again exports that one, and several photos are exported again through the sheet.
- **A notice for each export as it ends,** "Exported 12 photos to apertrust_exported" with Show in Finder, kept in the notices' history (the library's step 12), since the jobs window closes itself after a clean end.
- **Export Again** exports the window's selection at once with the `export.*` settings the sheet left, with no sheet, asking darktable's overwrite question.

### What the session adds

- **`PreviewExport` names the photo whose notes the sheet shows** (`focus`, the sheet's active photo): its name expanded in full and its output size, the open photo's from memory and another's planned from its saved edit over its raw's dimensions, read without decoding the image ([0056](0056-export-as-an-engine-job.md)).
- **`ExportPreview`'s list of each photo's file and what On conflict does** gives the cells their marks. A photo whose name the preview cannot give exactly, its path using what only a decode tells (the camera's name as the decoder reads it, the ISO, the exposure bias, the image's sizes), has no mark and is left out of the counts of files replaced or skipped; its note says its name is known once it is exported. Nothing else changes for a script: the same `export.*` settings, `Command::Export`, `CancelJob` and `RetryJob`.

## Options considered

- **The dialog as a column of export tools,** 640 points wide with a height rule for small screens: a second form beside Import's sheet, and no photos to check.
- **One tool page with darktable's section labels and no tool header:** nothing resets on its own, and a column unlike Import's.
- **The folder in the grid, the selection checked:** one photo drowns among hundreds, and reviewing the set needs Checked only.
- **Only the photos opened with, no toggle:** a forgotten photo means closing the sheet and selecting again.
- **The selection never written back:** Export Again would export a set other than the one just exported. **Written back on Close too:** closing would change the selection.
- **No built-in presets,** darktable's: the way to a small file, and back to full size, takes several rows each time. **An "Update earlier exports" preset,** Overwrite if changed: a replace in one click.
- **darktable's three labels whole as the titles:** "options" three times in a column of tools that all hold options. **A name of Apertrust's own for Global options,** such as Size and colour: it leaves out Style and Store masks, and renames a darktable term its users look for.
- **darktable's No and Yes choices for the three booleans:** two clicks for what a tool draws as a toggle of one.
- **The column's tools pinned, floated or moved to the side panel:** settings shown twice, the second time beside the photo's edit.

## Consequences

- `crates/app/src/export.rs` becomes the Export sheet on [0086](0086-import-and-export-as-one-sheet-pattern.md)'s frame: the five tools, each declared with the place Export; the grid of the set; Select edited; the marks; the alerts on the main window. The dialog's form, its host code for Export and its snapshots go then. The dialog is not rebuilt as a dialog of tools before.
- The session's `PreviewExport` gains `focus` ([0056](0056-export-as-an-engine-job.md)); `apertrust-export`'s presets gain the two built in.
- Selecting several photos (Phase 3) writes the selection after an export; the notices come with the library's step 12.
- `docs/gui.md` describes it ([The Export sheet](../gui.md#the-export-sheet)), with its deviations D1 to D4 and D15 to D17; the wireframe's Export board is redrawn on the sheet; the `ui-conventions` skill is to describe it.
