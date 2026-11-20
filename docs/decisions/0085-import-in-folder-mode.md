# 0085. Import in folder mode

## Context

Folder mode opens a folder as it is ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), but photos still have to get from a card or another folder onto the computer. darktable does it with copy & import (`src/libs/import.c`, `src/common/import_session.c`, `_control_import_job_run` in `src/control/jobs/control_jobs.c`): a dialog listing the source's files, its naming rules with variables, a job that copies, and a database field that remembers what was imported. Folder mode has no database and writes nothing but sidecars, so "already imported" needs another answer. Import is a full-window sheet of [0086](0086-import-as-a-full-window-sheet.md)'s pattern and a `UserForeground` job ([0057](0057-jobs-as-darktables-control-jobs.md)). The question is what the Import sheet holds, how photos already imported are told, what the copy guarantees, and how it is built in slices.

## Decision

**Import… in folder mode is darktable's copy & import, row for row under Apertrust's words, on the Import sheet** ([0086](0086-import-as-a-full-window-sheet.md)): the source's photos in a grid, checked to import, beside a column of import tools. Planned: Phase 3's step 20, in three slices (below).

### The sheet

- **Opened by** Ctrl+I (the `import` action), File › Import…, the Toolbar's Import…, Collections' Import… ([0090](0090-export-as-a-photos-tab-tool.md)) and the start window's Import…, after Open Folder…; enabled while no dialog or sheet is up. It works the same in one photo and in folder mode, never acts on what is open, and opens the destination as After import says, by default as the first photo lands.
- **On [0086](0086-import-as-a-full-window-sheet.md)'s frame:** the heading "Import" then the source and its counts ("EOS_DIGITAL · 412 photos, 3 imported before"); Select all, Select none and Select new; Checked only; Sort, by capture time ascending (darktable's) or file name; Zoom. The footer's summary "209 of 412 photos · 6.2 GB → ~/Pictures/Apertrust/20261115_Wedding", its second line "Copies only; nothing on EOS_DIGITAL is changed.", or in `warning` "380 of these were imported before; they will be copied again.", or why Import is disabled; Keep open; Close and **Import 209 photos**.
- **With Keep open on,** the checks clear once the import starts; each photo is marked imported as the job's `Imported` events name it, and the source is checked again when the job ends; the sheet lists the next card as one comes in. A photo of an import queued or running shows "Being imported" on its cell, its check box disabled and left out of Select new and Select all until its job ends, so a second press never copies it twice.
- **Presets of the whole sheet** ([0086](0086-import-as-a-full-window-sheet.md)), kept in `presets/import.toml`: Default settings, then two built in, **Original names** (Keep original file name on) and **By capture date** (the subfolder `$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(JOBCODE)` and the file name `$(EXIF.YEAR)$(EXIF.MONTH)$(EXIF.DAY)_$(SEQUENCE).$(FILE_EXTENSION)`), the rest of each at darktable's defaults; then the person's own. darktable ships none. darktable's own patterns are Default settings, so no preset repeats them.

### The tools, in their default order

| Tool | Rows, with darktable's defaults unless said | Slice |
| --- | --- | --- |
| **Import from**, open; closed while a card is the source, which the heading names | Places: cards and drives first (a card marked as one), Pictures, Home and the person's own places, with darktable's ＋, − and ↺; Folders, Collections' tree of the chosen place; Include subfolders (darktable's "recursive directory", off), on and disabled for a card, whose `DCIM` is always listed whole; Select only new photos (on). On a fixed disk, the note "These photos are already on this computer. To edit them where they are, use Open Folder…", with **Open Folder…**, which opens that folder and closes the sheet | 20a |
| **Import to**, open | Shoot name (`import.job_code`, `no_name`, the field's hint "Name of this shoot", its tooltip naming `$(JOBCODE)` and darktable's "import job"); Base folder (`$(PICTURES_FOLDER)/Apertrust`); Subfolder (`$(YEAR)$(MONTH)$(DAY)_$(JOBCODE)`); under them the notes: the sample path, the space left, the clashes; Override today's date last, cleared after each import | 20a |
| **Naming**, open | Keep original file name (off); File name pattern (`$(YEAR)$(MONTH)$(DAY)_$(SEQUENCE).$(FILE_EXTENSION)`), disabled while the original name is kept; the sample name | 20a |
| **Metadata**, closed | Initial rating (No rating by default), Ignore EXIF rating, Apply metadata, darktable's metadata fields each with "from XMP", Keywords | 20b |
| **Filters**, closed | Ignore non-raw files (off); Show already imported (on); Date taken later | 20b |
| **After import**, open; closed while a card is the source | Open the destination (When the import starts, darktable's and the default; When it ends; No); Eject the card and Verify the copies later | 20a |

- **The sources are the column's first tool,** so the grid keeps the rest of the sheet's width; the bar's heading names the source at all times. So a card's first view shows Shoot name and the sample path near the top.
- **The booleans are toggles,** as every sheet's ([0086](0086-import-as-a-full-window-sheet.md)), where darktable's import dialog has check boxes.
- **Backup to and Adjustments** are tools after Phase 3.

### The grid

- **The Browser's own parts,** as [0086](0086-import-as-a-full-window-sheet.md)'s cells: the camera's embedded picture, asked for only for the cells in sight (`LibraryCommand::SourceThumbnails { listing, photos, size }`, the `ListSource`'s number and the source's files by path, answered by `Thumbnail` events in that number and read with no folder open) and cached by fingerprint, so the destination's grid shows them at once.
- **Once the listing and the check end,** the photos not imported before are checked, or all of them with Select only new photos off.
- **A photo imported before** carries a mark at its picture's bottom right (Lucide's `circle-check`), its tooltip naming where its copy is, and is left unchecked; checking it copies it again, which the footer says. A source with nothing new says "All 412 photos on EOS_DIGITAL are already imported."
- **What is not done yet is said:** until 20b only raws are listed, and the heading counts what is left out ("40 raws · 860 other files not imported in this version"); with no source, the empty state adds "A phone connected by cable doesn't show here yet: copy its photos to a folder first."

### Already imported: told by the destination alone

No record is kept and nothing is written on the source.

- **Where it looks:** the base folder's whole tree, its pattern expanded up to its first variable that is not a fixed folder's (`$(HOME)`, `$(PICTURES_FOLDER)` and the like): `~/Pictures/Apertrust` and everything under it by default, so a card put back on another day, whose photos would go into another dated subfolder, is still recognised. The source's own folders are left out of the walk, so a source inside that tree never matches itself.
- **Candidates first:** a file in that tree of the source photo's main file's size and camera name, a raw's, or from 20b a photo's only image file, a JPEG alone among them: its own file name, or the camera name its `.apxmp` keeps in `xmpMM:PreservedFileName`, which every copy Import names otherwise than its source carries, one with `_01` added included. The tree is listed for names and sizes alone; a name is read from a sidecar only for a file whose size matches.
- **Then the fingerprint** ([0053](0053-finding-folders-and-photos.md)), the size and BLAKE3 over the first and last 64 KiB, read from both files, confirms each candidate; a source photo with no candidate is new without reading it. A card of 5,000 photos already imported reads about 640 MB from it, an estimate of 7 s on a UHS-I card; a new card reads nothing beyond its listing.
- **A match is a probable one:** two different photos of one name and size whose first and last 64 KiB agree would be taken for one. The mark only leaves the photo unchecked, shown, its copy named, so the person can check it; the copy never skips a photo on its fingerprint alone (below). A candidate whose fingerprint cannot be read without downloading it, a cloud file not on the disk ([0053](0053-finding-folders-and-photos.md)), leaves the photo marked as probably imported, unchecked and shown with the candidate's name, until that file is on the disk.
- **A photo of several files,** a raw and its JPEG from 20b, counts as imported once each of its files is there. One whose raw is there and whose JPEG is not counts as new for the JPEG alone, and its import copies the JPEG beside the raw's copy, under its name, never touching the raw.
- **What it does not know:** a photo moved out of the base folder's tree, or imported under another base folder, counts as new. Another computer whose base folder is the same share knows what this one imported.

### The copy

`LibraryCommand::Import { photos, settings }`, the photos checked, a job of kind `import` on darktable's `UserForeground` queue, cancellable, as `dt_control_import` ([0057](0057-jobs-as-darktables-control-jobs.md)).

- **One photo at a time, in the order of their paths,** as darktable's import job sorts them (`_sort_filename`), whatever the grid's sort, so `$(SEQUENCE)` numbers them in that order; from one source; a raw and a JPEG of one shot keep one name, as darktable's `dt_has_same_path_basename`.
- **Copy, never move, erase or overwrite:** each file streamed into a hidden temporary in the destination folder, then renamed into place by a rename that refuses to replace a file, so a copy is whole or absent; its modified time set to the source's. Nothing is written on the source.
- **A photo lands as one:** its sidecars and its JPEG first, its main file renamed into place last, its raw where it has one, else its only image file (20b), so a photo counts as landed once its main file is there. A photo whose main file did not land has the files it copied removed, as its temporary is; a name counts as taken where any of a photo's files is there, so a sidecar a crash left is never overwritten. The destination check and Try again skip each file that landed.
- **A name taken:** a sequence continues after the last file there ("Continues after 20261115_0214, already there"); under original names the copy gets `_01`, then `_02`. A photo whose file at its destination, under its name or one of its `_01` names there, is the same byte for byte, compared whole as the copy reads the source, is skipped; its sidecars, and from 20b its JPEG, missing beside that copy are copied alone under the copy's name, never over a file there. A sidecar there that differs from the source's, or from 20b a JPEG there of the copy's name that differs from the source's, is left as it is: the preview lists such photos among the clashes ("2 are already there with an edit of their own; their edits on EOS_DIGITAL stay there", "1 has a different JPEG there; it stays as it is"), and `Imported` names each one whose sidecar or JPEG was not copied. Any other checked photo is copied, one imported before included; the footer's count of photos copied again leaves out those that will be skipped.
- **Carried with each photo,** renamed with it: its `.apxmp` and `.apdata`, and darktable's `IMG.ARW.xmp` as it is ([0051](0051-importing-darktables-edits.md)). The source's sidecar is never written; the copy's names its new photo.
- **A copy named otherwise than its source,** by a pattern or with `_01` added, keeps its camera name in `xmpMM:PreservedFileName` in its `.apxmp`, as darktable writes it, which is what "already imported" finds it by: one sidecar for each such photo. Otherwise Import writes no sidecar unless Metadata sets something, written as ordinary steps noted Import. The initial rating is none by default, where darktable's is 1 star.
- **Failures,** as [0056](0056-export-as-an-engine-job.md)'s for export: a photo's own failure is that photo's and the import goes on; the source or the destination failing (the card removed, the disk full, a share dropped) stops it. The copies made stay. Try again (`RetryJob`) copies what is left. Cancel leaves the copies made and no temporary.
- **Problems are shown before Import,** each disabling it with its reason: an unknown variable, a folder that cannot be made or written, a destination that is the source or lies inside it, not enough space, a date darktable cannot read.
- **The destination opens** as the first photo lands (After import's default): as a way in and a recent, the folder Collections opens, with no photo opened. Where the patterns make several subfolders, their deepest common folder is the one opened, the first subfolder chosen in Collections' tree as a click chooses it, and the others list as they are chosen, each listing its own photos as any folder does. The session adds each photo copied into the folder the Browser shows to its index, so the Browser fills as they land, never reading the folder again. At the end, a notice: "Imported 214 photos into 20261115_Wedding" (step 12). There is no Undo; copies are removed with Delete (step 19).

### A card coming in

A card is a mounted volume with a `DCIM` folder at its top. While one with photos not imported is in, the Toolbar's Import… and Collections' ([0090](0090-export-as-a-photos-tab-tool.md)) gain a dot, each with the tooltip "EOS_DIGITAL: 214 new photos", and Import opens with it chosen. **When a card is connected** (`import.card_connected`: Do nothing, Mark Import, the default, or Open Import) lets Import open by itself. The volumes come from `apertrust-display`; counting a card's new photos is a `SystemBackground` job, never in the jobs window.

### What a script sees

- **The settings,** `import.*`: the job's own (base folder, subfolder, file name, original names, shoot name as `job_code`, date, rating and metadata, the source's options and After import's) with owner `import`, generated by `apertrust-library`, kept by the app and read by the CLI; the sheet's own (places, folder, and [0086](0086-import-as-a-full-window-sheet.md)'s column width, order, Keep open and Zoom) with owner `app`. Settings › Import shows darktable's four preference rows, the base folder, subfolder, file name pattern and Keep original file name, and When a card is connected.
- **The commands** (decision [0054](0054-four-layers-for-the-librarys-code.md)'s layer 4): `ListSource { source, include_subfolders }`, a cancellable `UserBackground` job of kind `import-source` with no progress, shown in the sheet alone, cancelled by the next `ListSource` or by `CancelListing { listing }`, its number, as the sheet closes, answered by `SourceListed` chunks and capture times, then `SourceChecked` against the base folder, checked again as it changes and when an import into it ends, a change cancelling the check still running, which sends nothing more, each check numbered within its listing and its `SourceChecked` naming that number and the base folder it checked, the sheet keeping only the latest check's; `PreviewImport { photos, settings }`, a `UserForeground` job with no progress as Export's preview, answered by `ImportPreview` (the sample paths and folders, the clashes, the photos imported before among those checked, the size, the free space and the problems); `Import`, with `Imported` chunks naming each source and its copy; `Volumes`, the cards and drives as they come and go (20c). Each event carries its command's number, the sheet keeping the latest `ListSource`'s and `PreviewImport`'s, and marking photos imported only from the `Imported` of an `Import` it sent from the listing it shows, so another import's events change nothing, a card of the same name included; the folder's filter on the latest `Open` does not apply to them.
- **The CLI:** `apertrust-cli import SOURCE... [--base-folder P] [--subfolder P] [--file-name P | --original-names] [--job-code X] [--date D] [--all] [--dry-run]`, only new photos unless `--all`, `--dry-run` printing the preview, with 20a.

### The slices

| Slice | Delivers | Needs |
| --- | --- | --- |
| **20a** | The ways in, Collections' Import… among them ([0090](0090-export-as-a-photos-tab-tool.md)); the Import sheet on the frame; the grid with its bar, embedded thumbnails, the mark, Select all, none and new, Checked only, Sort and Zoom; Import from, Import to, Naming and After import, with the first view above and the notes on what is not done yet; the destination check; `PreviewImport`; the copy job with its failures and Try again; the destination opened and filled as photos land; raws only; the column in its default order, ⋯ disabled; `apertrust-cli import` | Step 12's notices; step 19's copy job; the setting rows and the sheet frame ([0086](0086-import-as-a-full-window-sheet.md)) |
| **20b** | Metadata; Filters; a heading over each capture day with a check box for its photos; a raw and its JPEG as one cell, a JPEG left behind imported beside its raw; non-raws listed and copied | Steps 14, 15 and 17 |
| **20c** | The sheet's presets, the three built in; reordering the column; the card's dot and its setting; a larger look at one photo | Presets; [0083](0083-the-side-panels-layout-dragged-and-floating.md)'s moving |
| After Phase 3 | Verify the copies, through Move to…'s checked copy ([0101](0101-folders-first-libraries-optional.md)); Eject the card; Backup to; Adjustments with styles; Filters' Date taken | Step 19 |
| Later | Cameras and phones through libgphoto2, once its exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md) is decided | — |
| Never | Erasing a card, moving the photos, overwriting a file | Principle 3 |

### Deliberate differences from darktable

A sheet of tools beside a grid, not a list in a dialog; thumbnails always, for the cells in sight; already imported told by the destination, not a database field; no first-time alert about the base folder, since the sample path is always shown; the checks, not a selection; a card's `DCIM` listed whole; `_01` for a taken original name and an identical photo skipped, where darktable switches to the pattern; the sidecars and darktable's `.xmp` carried; problems shown before Import; never an overwrite; "Shoot name" for darktable's "import job"; Open the destination a choice; no initial rating; three presets built in, where darktable ships none.

## Options considered

- **A record on each computer** of what was imported, in the user data folder: a second truth beside the files, unknown to another computer and lost with the user data folder, while the destination says what is really there.
- **A hidden file written on the card:** writes on the person's card (principle 3), and cameras may object to it.
- **darktable's database field,** the file name and time: folder mode has no database ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)).
- **Import into a library or a project's capture folder:** no file is a library, so Import… copies into a folder in every case, and a Library adds a folder as it is ([0101](0101-folders-first-libraries-optional.md)).
- **Only the path the pattern would give:** a card put back on another day goes to another dated subfolder and looks new.
- **Fingerprinting every raw in the base folder's tree:** 128 KiB of every photo read on every import; the size and name narrow it first.
- **Whole files compared for the mark:** a 30 MB read of each candidate on both sides on every opening; the mark leaves the photo shown and checkable, and the copy compares whole files where it would skip.
- **Skipping a whole photo once its raw is there:** a JPEG left on the card by a raw-only import would never come.
- **The grid's sort or capture order for the copy:** two orders for one card's sequence numbers, where darktable's import job sorts by path.
- **Original names and a capture-date subfolder by default:** a departure from darktable that the presets give in one click.
- **"Job code" as the label,** after `$(JOBCODE)`: it says nothing about what to type.
- **darktable's 1 star:** a sidecar beside every photo copied, and imported photos unlike opened ones.
- **Nothing when a card comes in,** or Import opening by itself by default: the first is easily missed, the second interrupts an edit.
- **The sources in a column of their own,** as darktable's places and folders pane: three regions and a narrower grid.
- **darktable's folder clicked** for a card: a click into `100CANON` on every import.
- **darktable's switch to the pattern** for a taken original name, and copying an identical photo again: a surprise, and a duplicate.
- **darktable's `.xmp` left behind,** as its copy & import does: its ratings, labels and keywords lost to the copy.
- **Always opening the destination at the start,** as darktable: no way to import while working on another folder.
- **Non-raws from the first slice:** they need step 17's raw and JPEG groups first.

## Consequences

- `apertrust-library` gains the import: the source's listing, the destination check, the preview and the copy, through `apertrust_base::fs`; `ops` gains the commands and events above, served by the session as jobs.
- The fingerprint is also computed for the destination check's candidates ([0053](0053-finding-folders-and-photos.md)); the jobs gain the kinds `import-source` and `import` ([0057](0057-jobs-as-darktables-control-jobs.md)).
- The settings schema gains the owner `import` and the `import.*` keys with step 20a, and Settings' Import page its rows; `presets/import.toml` its three built in with 20c.
- The app gains the Import sheet on [0086](0086-import-as-a-full-window-sheet.md)'s frame; `apertrust-display` the volumes and, later, eject.
- `apertrust-cli` gains `import`.
- [library.md](../library.md#import-in-folder-mode) describes the library's side, [gui.md](../gui.md#the-import-sheet) the sheet, [settings.md](../settings.md) the settings and [roadmap.md](../roadmap.md#the-librarys-steps) the slices; the wireframe's Import board is drawn on the Sheets board's frame.
