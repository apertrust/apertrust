# 0110. The Metadata tool: groups after EXIF and IPTC, metadata as values beside the edit

## Context

[0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md) makes Metadata one tool of the Photos tab, darktable's image information (`src/libs/metadata_view.c`) and metadata editor (`src/libs/metadata.c`) in one. This decision settles:

- which groups and rows it shows;
- how darktable's fields map onto the standards an agency, an archive or a client reads;
- where the values are kept, how they undo, and whether a rating, a keyword or a field is a step of the edit's history;
- what the tool's presets hold;
- how it reads with several photos selected.

**What constrains it:**

- darktable's fields are the nine of its metadata table (`src/common/database.c:3236-3244`), Dublin Core's and its own, a notes property in another editor's namespace among them. Its metadata is undone by its global undo, never its history stack (`DT_UNDO_METADATA`, `src/common/metadata.c:771-781`).
- Two computers combine metadata field by field ([0052](0052-comparing-two-copies-of-a-sidecar.md)).
- A utility holds the photo's metadata, never controls in the edit ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)).
- A tool takes no right-click (AGENTS.md), and a panel holds no free text, its notes in tooltips and its problems as warning buttons ([0086](0086-import-as-a-full-window-sheet.md)'s rule for the Import tool).

The design is drawn on the [UI wireframe](https://claude.ai/artifact/V4r2xReSqn344AQstUgeiK#artboard-MetadataTool)'s board "Metadata tool · clean · proposal". [gui.md](../gui.md#the-metadata-tool) holds every row.

## Decision

**One utility, Metadata,** id `metadata`, in the Photos tab between Keywords and Styles ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)), closed at first. It is darktable's image information and metadata editor in one: it shows what the file and the camera recorded, and edits what the person writes. **It follows the selection, never the pointer:** it describes every selected photo, which is the active photo where one alone is selected. Hovering a thumbnail changes nothing, where darktable's image information follows the image under the pointer first (`src/libs/metadata_view.c:496-536`). Planned.

### The groups

**Eleven collapsible groups, following the standards.** They run from what the file is, through what the camera recorded, to what the person writes, with the maker's own block last.

| # | Group | Follows (the title's tooltip) | Rows | Open at first |
| --- | --- | --- | --- | --- |
| 1 | Basic | The file and the library | Read-only, but Rating, Colour labels and the copy's name | Yes |
| 2 | Camera | EXIF · IFD0 | Read-only | No |
| 3 | Exposure | EXIF · Exif IFD | Read-only | Yes |
| 4 | Lens | EXIF · Exif IFD, the lens tags | Read-only | No |
| 5 | GPS | EXIF · GPS IFD | Read-only | No |
| 6 | Content | IPTC Core · Content | Editable | Yes |
| 7 | Creator | IPTC Core · Contact | Editable | No |
| 8 | Rights | IPTC Core · Status, the rights | Editable | No |
| 9 | Location | IPTC Core · Image, the location | Editable | No |
| 10 | Status | IPTC Core · Status | Editable | No |
| 11 | Maker notes · <maker> | The camera maker's MakerNote | Read-only: the rows Apertrust decodes, then "N fields · Show all" | No |

- **Plain names** in the panel; the standard a group follows is in its title's tooltip, and each row's tag or XMP property in that row's tooltip. There is no darktable group.
- **Basic** is the file and the library: File name, Folder, Type, Size on disk, Dimensions, Rating, Colour labels, Edited, the virtual copy's name and number, then, hidden at first, Preserved file name and Export size.
- **Title sits in Content** beside Description, though IPTC files it under Status as a short reference: darktable and most people use it as the photo's title. **Publisher,** Dublin Core and not IPTC Core, sits in Rights beside Credit line and Source.
- **Most rows are hidden at first.** Those shown at first are, with one photo: Basic's nine (Copy only where the photo has several copies), Exposure's eight, Content's three (Title, Description, Notes); in closed groups, Camera's Make and Model, Lens's Lens, Focal length and 35 mm equivalent, GPS's Latitude, Longitude and Altitude, Creator's Creator, Rights' Copyright notice and Publisher, Location's Sublocation, City and Country, and Maker notes' decoded rows. ＋ shows or hides any row.
- **A group with no row shown is not drawn:** Status at first. Maker notes is not drawn for a maker whose notes Apertrust does not decode.
- **Facts and fields look different.** A read-only row, a fact (EXIF, maker notes, what the file and the library know), is a label and plain text, as Export's Summary lines are. An editable row, a field (the IPTC groups' fields, Notes, the copy's name), is a label and a well filled `sunken`, drawn by fill alone, showing its editing outline only while it is edited. Rating and Colour labels are star and dot buttons.
- **Keywords** (`dc:subject`) are the Keywords tool's, right above, never repeated here.

### darktable's fields

| darktable's field | Here | Group |
| --- | --- | --- |
| Title, Description, Creator, Rights | IPTC Core's own Dublin Core properties, `dc:title`, `dc:description`, `dc:creator` and `dc:rights` (Copyright notice), read and written as they are | Content, Creator, Rights |
| Publisher | `dc:publisher`, kept, so darktable still reads it; never folded into Credit line, which means something else | Rights |
| Notes, in another editor's namespace | **Apertrust's own `apertrust:notes`,** one per virtual copy, private: left out of an exported file unless Export's Metadata group includes them; darktable's notes read into it | Content, last |
| Version name, `darktable:version_name` | **The virtual copy's name,** which the sidecar already holds ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md)), shown and edited in Basic as "Copy: Black and white (2 of 3)"; darktable's read into it; offered to Export's file-name variables as darktable's `$(VERSION.NAME)` | Basic |
| Preserved filename, `xmpMM:PreservedFileName` | Read-only, since Import's already-imported check relies on it ([0085](0085-import-in-folder-mode.md)) | Basic, hidden at first |
| Image id | Not read: darktable's database | — |

- **darktable's `.xmp` is read** for these fields, as for its ratings, labels and keywords, while a photo has no `.apxmp`, and all of them are copied into the `.apxmp` the first time Apertrust writes one ([0051](0051-importing-darktables-edits.md)); built for the rating and labels, the rest with the library's step 15.
- **An empty field shows the camera's own text,** as darktable's import takes it (`src/common/exif.cc:807-830`, `:1977-2007`): Description from EXIF ImageDescription or IPTC Caption, Creator from EXIF Artist or IPTC By-line, Copyright notice from EXIF Copyright or IPTC Copyright, its tooltip naming the source. Nothing is written until the person changes it; clearing it then writes an explicit empty, so the camera's text does not come back.
- Fields belong to a virtual copy, as darktable's belong to a version.

### Values beside the edit, never steps

**Metadata are the photo's current values, not steps of the edit's history.** The same holds for the rating, the reject, the colour labels and the keywords.

- **Kept** per virtual copy in the sidecar, beside its history: each field its value and the hybrid logical clock of its last change ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md)), an explicit empty kept apart from a field never set; each keyword its presence on the copy with the clock of its last change, a keyword taken off kept as absent with its clock, so a removal is not undone by an older copy. Each change advances the copy's clock vector, as a move of the log's end does. The properties are the standard ones named above, `darktable:colorlabels` as darktable writes it, and the keywords in `dc:subject` and the hierarchical subject property as darktable writes them ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)). The stars and the reject are two values, each with its clock, so a rejected photo keeps its stars: entries of the copy's `apertrust:fields`, `xmp:Rating` the stars, kept while rejected, and `apertrust:rejected` the reject, beside `darktable:colorlabels`. A reader wanting darktable's single rating takes −1 where the reject is set, as darktable writes it (`src/common/image.c:600-605`); writing them at the file's top level for other programs is the library's step 31.
- **Combined field by field and keyword by keyword:** two copies of a sidecar take, for each field, the value with the later clock, and for each keyword, its presence or absence with the later clock ([0052](0052-comparing-two-copies-of-a-sidecar.md)). Metadata changed on one computer and the edit on another never splits a photo into two virtual copies. A clock ignored for being far ahead still orders where it is, as [0047](0047-where-edits-live-and-what-the-sidecar-holds.md) has it for steps: a field changed after reading it orders before it until the wall clocks pass it.
- **Undone with Undo.** A change is one undo record of the session's, holding each field's value before and after it on each photo, which Undo puts back and Redo sets again. Undo takes back the newest record, a metadata change's or the edit's, in the order they were made, and Edit › Undo names it ("Undo Title on 12 Photos"). A change on several photos is one batch, which one Undo takes back on all of them; a photo whose field changed since keeps its later value, and the notice names it. Redo sets again only the fields Undo put back, so it never overwrites a later change either. **A record is never saved:** the sidecar holds the values, not a log of their changes, so it holds nothing to undo from. A change of one photo is a record of that photo's, ending as it closes, as its edit's records do; a batch's lasts while the app runs, as [0109](0109-a-style-on-every-selected-photo-one-batch-one-undo.md)'s batches do. After a restart a metadata change cannot be undone.
- **Consecutive commits of one field,** within the session's merge times, are one record, as a control's quick repeats are one step ([0078](0078-undo-redo-and-the-edits-history.md)). Any change between breaks a merge both ways: an edit change between two commits of a field, and a metadata change between two changes of a control, so Undo always takes changes back in the order they were made.
- **Never in the History tab.** Reset, Go to a step, Compress, Truncate and Discard History never touch them. Copy Edit and styles carry none.
- **Keywords** are set and changed by the Keywords tool ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)), stored as above; a keyword put on or taken off is one undo record as a field's change is, and a change to the keyword list (rename, move, delete) one record over every photo it rewrites.

### Rating, reject and colour labels

**They are global actions, the app's own as darktable's, owned by no tool** (`rating-0` to `rating-5`, `reject` and the label actions; [gui.md](../gui.md#the-caption-under-the-photo)). Their keys, the caption under the photo, the Browser's cells, the Photo menu and a thumbnail's right-click menu all call them; the Toolbar offers none of them. Basic's Rating and Colour labels rows call the same actions: one more way to them, not their owner. With several photos selected, Rating reads Different with its ⌄ list of ratings and counts, and a label's dot is filled where every photo has it and ringed where some do; a click on a star or a dot does what the action's key does, a label following the active photo ([gui.md](../gui.md#selection-and-the-active-photo)).

### Presets

**The header's presets button, as every tool's** ([0106](0106-presets-and-styles-as-appreset-and-apstyle-files.md)). A Metadata preset stores the tool's view settings, which groups and rows are shown and which groups are open (`metadata.shown`, `metadata.hidden` and `metadata.open`, as `setting:` values in an `.appreset`), and never holds a field's value or writes a photo.

- **The pull-down:** Default settings, the rows shown at first with Basic, Exposure and Content open; the built-ins, Essentials (a short set for culling), Everything and IPTC for agencies, each a file in `data/presets/metadata/`; then the person's own.
- **＋, Rows Shown,** lists the groups, each with its rows checked where shown, as Filters' ＋ lists its groups ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)). A row hidden keeps its value and stays in exported files, Notes aside. The groups set the order; rows are not reordered.
- **Field-value presets** (a Creative Commons licence in Rights, Creator's contact details) come later.

### Several photos

- **A shared value shows; a value they differ in reads Different,** or a range or a total where it helps: Date taken, Exposure time, Aperture and ISO as ranges ("08:10 – 09:42", "1/2000 – 1/60 s"), Size on disk as a total ("384 MB in all"), File name as a count ("12 photos").
- **Typing in a field writes to every selected photo,** one batch; Return with nothing typed over Different writes nothing, darktable's "leave unchanged" (`src/libs/metadata.c:362-367`).
- **The ⌄ at a Different field's end** lists each value with its count, most common first, at most ten, then **Clear on 12 Photos**. A value chosen is written to every photo at once, one batch.
- **Computed off the interface thread:** the session answers `Command::Summarise` with `Event::Summary`, built by the library's `summary::build` in a job from what the session knows of the photos, their listing and their facts, kept in its selection's collection. A photo's facts are one parse of its raw's metadata with no decode (`Shot`), read by that job when a summary first needs them, never by the folder's open, whose capture times come from the EXIF alone: on an SSD the full parse costs 6 to 30 times the EXIF read ([library.md](../library.md)). A saved cache of them is planned with the saved index (Phase 5). Only the latest selection's summary is ever shown; until it comes the values are blank, then "…" after 150 ms. A write to many photos is a cancellable job reporting its row ([0057](0057-jobs-as-darktables-control-jobs.md), [0072](0072-each-job-reports-its-own-row.md)), the fields showing the new value at once.
- **Through the session:** a library command writing a field on photos, a batch of source You or Assistant, through `LibraryCommand` and `LibraryEvent`, and the selection's summary, the session's `Command::Summarise` answered by `Event::Summary` ([0054](0054-four-layers-for-the-librarys-code.md)), so a script or an assistant reads and writes what the tool does (principle 8). Their names are chosen with the slice that builds them.

### Interaction

- **No Apply.** Return, Tab or the focus leaving commits a field; Esc puts back that one field as it was when editing began. A field captures the photos it edits as editing begins, so a selection changing mid-edit commits to those. Line breaks only in Description and Notes (Shift+Return or Ctrl+Return); spaces trimmed at both ends, as darktable's (`src/common/metadata.c:402-417`).
- **No right-click.** A read-only row takes the focus; Ctrl+C or Edit › Copy copies its value. ⋯ holds Copy Information (every row shown), Copy Coordinates, and Clear Metadata Fields on N Photos…, darktable's metadata editor reset, asking first with the count, one batch; the header has no ↺.
- **No text in the panel.** Reasons and sources are in tooltips. Problems are warning buttons: a failed write on its field, with the reason and Try Again; view-only photos among the selected on Content's title row, with Select Only Editable Photos.
- **Basic's Folder row** ends with Open This Folder, darktable's jump to the film roll (`src/libs/metadata_view.c:1156-1161`, Ctrl+J).

### The order of work

One slice at a time, each reviewed before the next. Slice 1 is built, and of slice 2 the read-only rows of several photos and the presets; Maker notes and slices 3 to 5 are planned:

1. **Read-only for one photo:** the tool, Basic (less the copy's name), Camera, Exposure, Lens and GPS for the active photo from the session's `Photo` and `apertrust-raw`'s EXIF; the IPTC groups drawn closed and disabled; ＋, Ctrl+C and Copy Information. It needs none of the library's steps.
2. **Several photos, read-only:** Different, ranges and totals from the session's summary (`Summarise`, `Summary`), the facts read when it needs them; Maker notes for the makers rawler reads; the presets.
3. **Storage and editing for one photo:** the values beside the edit with their clocks, darktable's fields and the copy's name, the camera's text, failed writes and view-only photos; with the library's steps 14 and 15.
4. **Editing several photos:** typing over Different, the ⌄ list, Clear on N Photos, batches and jobs.
5. **Export:** Notes in exported files only where Export's Metadata group includes them, and the copy's name as a file-name variable.

The rest of IPTC Core beyond darktable's fields (Headline, Location's other rows, Rights usage terms and so on) follows slice 3; field-value presets, custom XMP fields, copying metadata between photos and editing the GPS coordinates, with the map, come later.

## Options considered

- **Decision 73's five groups** (Photo, Shot, Description, Location, More): groups of Apertrust's own invention, which a photographer reading IPTC or EXIF elsewhere has to translate; the standards already give the structure.
- **darktable's layout with an Apply button:** typed text is lost unless Apply is pressed before leaving the panel, darktable's best-documented complaint about its metadata editor (darktable issue 13829); every change saves as it is made (principle 3).
- **Metadata, ratings and keywords as steps of the edit's history,** one per field or keyword per photo: the history lists the photo's look, and a title is not part of it; Reset, Compress and Discard History would have to step around them; darktable keeps metadata out of its history too.
- **Ratings owned by the Metadata tool:** the keys, the caption and the menus would act through a tool that may be hidden or closed; darktable's ratings are global actions.
- **Notes mapped to IPTC Instructions:** Instructions is for whoever receives the photo, so private notes would leave with every export.
- **Version name as a metadata field:** a virtual copy already has a name; two names for one copy would drift apart.
- **Field-value presets now,** as darktable's metadata editor's presets: a tool's presets button holds that tool's own settings ([0106](0106-presets-and-styles-as-appreset-and-apstyle-files.md)); presets that write library data to the selection need a design of their own.
- **Following the hovered thumbnail,** as darktable's image information does: the panel would change as the pointer crosses the Browser, and a field being typed in would have no stable target.

## Consequences

- **Decision 73's Metadata bullet** names these groups and this decision. **Decision 51** reads darktable's metadata fields too and copies them in at the first write; the library's steps 14 and 15 store ratings, labels, keywords and metadata as values beside the edit, never steps ([roadmap.md](../roadmap.md#the-librarys-steps)). **Decisions 47, 52 and 85** and [history.md](../history.md) no longer plan ratings, labels, keywords or metadata in a step; an import's ratings, keywords and fields are written as values.
- **`apertrust-tools`** gains the tool's declaration: its groups and rows as data, each with its tag or property, whether it is editable and whether it shows at first, and row kinds for a fact and a field that a utility's page may hold and a processing tool's may not.
- **`apertrust-ui`** gains a fact list shared with Export's Summary and a wrapping text field (one to six lines) with Different, its ⌄, a warning button and the editing outline.
- **`apertrust-library`** gains the values and their clocks in the sidecar's version 2, combining field by field and keyword by keyword, the summary of several photos' facts (`summary::build`), a photo's facts read on demand (`Shot`), the index's camera, lens and exposure ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)'s consequence) planned with the saved index, and file sizes.
- **The engine's session** gains a metadata change's undo record beside the edit's.
- **`apertrust-raw`** reads IFD0's, the Exif IFD's and the GPS IFD's tags the rows show, with offsets and sub-seconds, and the maker notes' decoded rows from what rawler reads.
- **`apertrust-export`** leaves Notes out unless the Metadata group includes them, and expands `$(VERSION.NAME)` to the copy's name.
- **The docs:** [gui.md](../gui.md#the-metadata-tool) holds the tool, its rows and its states; [library.md](../library.md) the storage; [history.md](../history.md) what is not a step; the roadmap the slices.
