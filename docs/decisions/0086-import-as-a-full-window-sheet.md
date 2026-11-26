# 0086. Import as a full-window sheet

## Context

Import… chooses photos on a card or in a folder that is not open, and holds settings beside them. darktable draws copy & import as a dialog listing the source's files with its options beside them (`src/libs/import.c`). Import is one of [0037](0037-one-view-the-photos-tab.md)'s full-window sheets; its photos are not the open folder's, so the Browser and its selection cannot show them. Export, which acts on the selection, is a tool of the Tools panel instead ([0090](0090-export-as-a-photos-tab-tool.md)). The questions are what the sheet is made of, how its photos are chosen and looked at, what its column holds and where its presets are, what the footer says, its keys and modality, and which tools may appear where.

## Decision

**Import… opens a sheet:** full-window inside the main window, below the Toolbar, drawn with the design system's components on `panel`, the same on every platform, never one of AppKit's sheets. A sheet bar across its top, the Browser's grid with check boxes, a column holding one tool, Import, beside the grid, and a footer; its content is [0085](0085-import-in-folder-mode.md)'s. The frame is general, so Print may take it when it is designed. Planned: built with Import's first slice, in the order below.

### The regions

| Region | Where and how big | Holds |
| --- | --- | --- |
| **The sheet bar** | Across the whole sheet, as tall as the viewer bar, its 2-point separator under it | The heading, the sheet's name and its source ("Import · EOS_DIGITAL") in 16-point Medium, then its counts in `text_secondary`; the Select buttons; Checked only; Sort; Zoom; the hint "Hold W to look closer", with the larger look |
| **The grid** | The rest of the sheet's width, between the bar and the footer | The Browser's grid component, its cells, Sort, Zoom and keys, with check boxes as its choosing, and day headings |
| **The column** | 440 points by default, resizable at its edge from 296 to 600, and to no more than half the sheet where that leaves more than 296: 296 is the floor, below a 592-point sheet too; kept (`import.column_width`) | The Import tool, scrolling with the quiet scroll bar in its end margin |
| **The footer** | 52 points across the sheet, a hairline above | A Help button at its start, as Settings' footer has, opening the topic `import` ([0097](0097-help-topics-and-the-apertrust-guide.md)); the summary on two lines; Keep open, Close and the primary button at its end |

- **The column sits on the Tools panel's side,** the right today, the left when the Tools panel moves there, and the grid on the Browser's: opening a sheet, thumbnails appear where thumbnails were and a tool where tools were, and reading down the column ends above the primary button.
- **Narrow windows:** the column gives way only once the grid is down to two cells at its Zoom, then down to 296, and never below it: in the least window, the planned 500 points wide (decision 39; today 960 logical pixels, 960 points at the default interface size and fewer at a larger one), the column is 296 and the grid the 204 left, two Small cells; the Select buttons fold into one Select ▾ below about 520 points of grid; the summary's path is cut first. The bar and the footer always stay in view, and Close and the primary button are never cut.

### The grid and the checks

- **The grid is the Browser's grid component,** its cells, Sort, Zoom and keys, with check boxes as its choosing: the picture fitted in its square; a caption always shown, the check box then the file name cut in its middle; the sheet's active photo's ring and no selected ring; at the picture's bottom right on the thumbnail scrim, a destination mark on a photo imported before, whose picture is dimmed while unchecked. Only the cells in sight are drawn.
- **Day headings,** while sorted by capture time: a heading over each capture day with its counts and a three-state check box for its photos, checked, unchecked or some.
- **At opening** the new photos are checked, or all of them with Select only new photos off, and the first of them is the sheet's active photo, or with none checked the first photo listed.
- **The checks are what goes,** not a selection, so a stray click never drops a large choice. A click on a check box, or Space on a focused cell, turns its check on or off; a click on the picture makes it the sheet's active photo, whose values the rows' tooltips follow, and leaves the checks; Ctrl-click (Cmd on a Mac) on a picture turns its check and makes it active; Shift-click on a picture checks the range from the active photo to it, and on a box, or Shift+Space, sets the run from the last cell touched as that one was set.
- **The sheet's active photo is its own:** it never changes the window's active photo or selection.
- **The bar's Select buttons,** quiet text buttons as darktable has them: Select new, Select all and Select none; then Checked only, a toggle that hides the unchecked cells.
- **The larger look:** while W is held, darktable's lighttable preview key, the active photo's picture fills the grid's area; ← and → move to the next photo, Space checks it; letting go shows the grid at the photo last shown.
- **The checks** are plain data (`Checks`: the checked set, the active photo, the last touched), unit-tested.

### The column: one tool

- **The column holds one tool, Import,** a utility drawn by every tool's header and page drawer from its `UtilityDefinition` ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)): its header, 30 points, the title, ↺, the presets button and ⋯, as every tool's; no on switch and no peeking with H, since nothing in a sheet edits a photo. Its parts are collapsible groups (`CollapsibleGroup`), each opening and closing on its own, its state kept as a tool's groups are; a closed group shows a short summary after its title ("No rating").
  - **↺** applies Default settings: every setting the tool owns but Source's place and folder, which it leaves as they are, to darktable's default, Folder to the default folder (`import.default_folder`, Settings › Import's Base folder), written at once.
  - **The presets button** holds the tool's presets (below).
  - **⋯** holds the tool's own items, declared as every tool's (`ToolDefinition::menus`), opened from its button alone, the header taking no right-click ([0091](0091-context-menus-places-and-tools-items-as-actions.md)); drawn disabled while none applies.
- **Every row that sets something is a tool setting's row** (`Row::Setting`'s look), beside action rows: choices Blended, darktable's combobox as a tool draws it ([0042](0042-numbers-shown-typed-and-stored.md)); a boolean a toggle (`ToggleRow`), where darktable has a combobox of No and Yes; no kept marker on any row, sliders included; no history step, a change being a setting written at once, Undo in a field undoing typing only; a slider reset as a tool's, by a double-click on its label or track or Delete; a row or an option not built drawn disabled, saying why in its tooltip, never hidden ([0030](0030-controls-as-darktables.md)).
- **No free text in the tool:** no note rows. What the session says about a row is its tooltip and its accessible description; a disabled row's reason is its tooltip; a problem is a **warning button** at the row's end, the warning sign in `warning`, or `error` where it stops the job, its tooltip saying what is wrong and its menu holding the fix.
- **The rows the tool panel lacks** are drawn in its language from its parts, each added to the design system with gallery snapshots, shared with the Export tool ([0090](0090-export-as-a-photos-tab-tool.md)): a slider row with no kept marker, its place left empty; the path row, the label, `PathField` stretching with darktable's variable list, a ⌄ of choices where the row has one and the folder `IconButton` at the end, in the Import tool its label on a line of its own above the field; the warning button; field rows, a label and a value field or a 180-point text field at the end; the size row, two or three 54-point value fields with ×, the unit, @ and dpi; an indented toggle, 20 points in.
- **Its states:** a group or a row not built is drawn disabled, a group closed, "Comes in a later version" in its tooltip; a problem shows its warning button, and one that stops the job turns its field's text `error`, with no outline round the field, which is drawn by its fill alone as every field; a closed group with a problem opens and scrolls into view once, as the problem appears.

### Where a tool belongs

- **Every tool has one place,** declared in its `ToolDefinition`, the parent every tool's definition embeds, a utility's as a processing tool's ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)): the Tools panel (the processing tools and the utilities, Export among them, [0090](0090-export-as-a-photos-tab-tool.md)) or the Import sheet, whose one tool is Import.
- **The Import tool never appears in the Tools panel.** Its dispatcher, the workspaces, built in or the person's, the visual editor, a header's drag, ⋯'s Move Tool to Tab, a drop on a tab's icon, the Hidden tab and Find Tool… take only the Tools panel's tools; a workspace's file that names a tool of another place skips it, as [0083](0083-the-side-panels-layout-dragged-and-floating.md) skips an id the build does not know, and never places it as a new section ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)).
- **The Tools panel's tools never appear in the sheet,** and its column draws only the Import tool.
- **Ids are unique across the places;** the Import tool's Metadata is a group of it, not a tool, so the Tools panel's Metadata is the only tool of that name.
- **A test** walks every tool and checks that each built-in workspace, the Hidden tab and search's list hold only tools of their place and the sheet's column only Import, and that every layout operation given the Import tool leaves the Tools panel unchanged.

### Presets: the Import tool's

- **The Import tool's presets hold the settings the tool owns:** never the source (Source's place and folder) or the checks, nor the sheet's own settings, Keep open, the column's width and the grid's zoom. They come from its header's presets button, as every tool's, with the presets API every tool shares.
- **Its list:** Default settings first, built in, setting the tool to darktable's defaults; then the built-in presets ([0085](0085-import-in-folder-mode.md)); then the person's, a ✓ on the one matching; then Save as Preset…, Update, Rename… and Delete…, darktable's "delete preset?" and "update preset?" as alerts. Pointing at a preset shows its settings in the tool, with a banner under its header, and the footer's summary follows, until the pointer leaves the list; ↑ and ↓ preview, Return or Space chooses.
- **Until presets are built,** the presets button is drawn disabled, its tooltip saying presets come in a later version.

### The footer

- **The summary,** two lines. The first, in `text`: the count, then the size or the format and size, then the destination cut in its middle ("209 of 412 photos · 6.2 GB of 412 GB free → ~/Pictures/Apertrust/20261115_Wedding, a new folder"), so what a tooltip says is never the only way to see it (decision 94's Touch mode): the free space, the destination with the date it carries, and whether it is a new folder, several (", 2 new folders by the day taken") or an existing one (", already there: 86 photos"); its tooltip the destination whole and the disk. The second, smaller: while the primary button is disabled, why; else what may surprise before anything happens, in `warning`, photos imported before that will be copied again; else the sheet's reassurance ("Copies only; nothing on EOS_DIGITAL is changed."). With nothing checked, "No photos checked".
- **Keep open** (`import.keep_open`), a toggle before the buttons, off by default and kept, as darktable's import dialog has "keep this window open" by its buttons: with it on, the sheet stays after the job starts, and the second line confirms each start ("Started: 40 photos → …/client").
- **Close and the primary button** in the platform's order (`dialog_host::buttons`). Close, not Cancel: the settings were kept as they changed, and Cancel would read as cancelling a job a press started.
- **The primary button** ("Import 209 photos") is disabled while nothing is checked, while Import's listing or its destination check runs, or while a problem stands; it stays focusable, its reason in its tooltip, in the footer's second line and as its accessible description.

### Keys, modality and screen readers

- **The sheet opens** with the keyboard focus on the grid's active cell and no focus ring until a key is pressed; with a card as its source, in Import name with its text selected, unless Import name is disabled, when it stays on the grid's active cell ([0085](0085-import-in-folder-mode.md)).
- **Return presses the primary button** wherever no text field is being typed in, no list is open and no button, choice or slider has the focus: a focused button, Close among them, presses itself, and a choice or slider opens its list or fine-control popup; a focused cell included; Ctrl+Return (Cmd+Return) from anywhere but an open list. Neither does anything while it is disabled.
- **Return in Import name,** a one-line text field, commits it and presses the primary button while it is enabled, so a card's import is the name typed and Return. In a path field, Folder, Subfolder or File name, Return commits the field, or takes the variable chosen in its open list, and presses nothing.
- **Esc,** Cmd+. and Cmd+W close the innermost thing first (a list, the variable list, a field being typed in), then the sheet, keeping its settings and dropping its checks, as Close does.
- **In the grid:** the Browser's keys, the arrows, Home and End moving the focus and the sheet's active photo; Space checks; Shift+Space sets a run; Shift with the arrows checks the cells passed; Ctrl+A and Ctrl+Shift+A are Select all and Select none; W held is the larger look. **F6** and Shift+F6 move between three regions: the bar and the grid, the column, the footer. The rows' keys are a tool panel's.
- **Modality:** while the sheet is open the view's keys and the Toolbar's actions rest, drawn at rest, and the menus that act on the view are disabled; the jobs spinner and View › Jobs stay live. One sheet at a time, and no dialog opens over one but the alerts it asks, which are the main window's. Closing the window closes the sheet first.
- **Screen readers:** the sheet is a region named by its heading under a dialog-role node; the grid a list named by its source and counts ("EOS_DIGITAL, 412 photos, 209 checked"), each day heading a group named by its day, each cell named by its file, its check box "Import DSCF1033.RAF", its mark in its description; the larger look announced by the photo's name, time and check; the Import tool as a tool in the Tools panel, each row's tooltip its description; a problem announced once, assertively, and the count politely, at most every 0.5 s.

### The build

1. **The setting rows,** in `apertrust-ui`, each in the gallery with snapshots, shared with the Export tool, the warning button and the path row's ⌄ among them.
2. **The sheet frame, with Import's first slice** ([0085](0085-import-in-folder-mode.md)'s 20a1): `Sheet` in `apertrust-ui` with its regions, the Browser's grid with check boxes as its choosing and the mark slot, `Checks`, the column drawing the Import tool from its definition with the place and its test, the footer with Help, Close and the primary button, the keys and the app's modality.
3. **With 20a2:** day headings, Checked only, Sort, Zoom and Keep open.
4. **The rest:** presets (20c), the larger look (20b), Import's later slices.

## Options considered

- **Import as a dialog,** darktable's: a list of files in a window of ordinary size, too small for thumbnails of a card's hundreds of photos.
- **Import as a tool, the card's photos in the Browser:** the Browser shows what is open, and a source's listing there would mix the two, checks with the selection ([0090](0090-export-as-a-photos-tab-tool.md)).
- **A grid of the sheet's own from the Browser's cells:** a second sort, sizes and keys to learn and to keep in step with the Browser's.
- **Several import tools** (the source, the destination, naming, metadata, filters, after import), each with its header, ↺ and a ⋯ holding only Move Up and Move Down: six headers for one job's settings, a reordering nobody needs in a column read once from the top, and presets with no tool to hold them. One tool with groups has one header with ↺, presets and ⋯, as every tool and the Export tool.
- **A presets row in the sheet bar** over the column, or presets as the column's first section, or a Presets column: a presets control unlike every tool's header, or 110 points of the column, or a grid 200 points narrower.
- **Notes under the rows:** free text that moves the rows below it as it changes and wraps in Ukrainian and Polish; tooltips, warning buttons and the footer say the same.
- **Every unchecked photo dimmed:** a new photo unchecked by hand looks like one imported before; dimming only those imported before keeps the mark's meaning.
- **The column always at the window's start,** the left in a left-to-right language: the grid jumps to the other side from the Browser as a sheet opens, and the column ends far from the primary button. Moving the Tools panel to the left gives that picture in every sheet.
- **Keep open as a row of the tool:** a setting of the sheet's, not of the copy.
- **Cancel to dismiss:** it reads as cancelling the job a press started; nothing in the sheet is undone.
- **Return in Import name only committing it,** Ctrl+Return the one key that imports from there: a card's import would end on a key nobody guesses, and a screen-reader user typing the name and pressing Return would hear nothing happen. **Return pressing Import from a path field too:** Return there takes a variable from its list.
- **A field outlined in `error` for its problem:** a line on a control drawn by its fill alone, the editing outline the one line a field draws; the warning button carries the problem, and the field's text says which.
- **The checks kept** for the same card while it stays in, this run, restored at the next opening: possible later ([gui.md](../gui.md#open-questions)); for now Close and Esc drop them.
- **The window's selection model for what goes:** a plain click selects one alone, dropping hundreds of chosen photos.
- **The column the Tools panel's width at opening:** at 320 points the default paths are cut in their rows, Folder's showing none of the part that varies, and the Polish print size row of the shared rows needs 407.
- **Return or a double-click for the larger look:** the sheet opens on a cell, so the one-key import would be lost; W held is darktable's key for the same look.
- **One of AppKit's sheets or a native window:** the grid needs the window's size, and a sheet drawn the same on every platform needs no host code.

## Consequences

- `apertrust-ui` gains the setting rows, the warning button, `Sheet`, the check-box choosing, the mark slot and day headings on the Browser's grid, and `Checks`; `TextField` and `PathField` draw a problem's field with its text in `error` for the Import tool, where their built error outline serves the Export tool; the app gains the sheet's modality and the Import tool's declaration.
- `ToolDefinition` gains the place, the Tools panel or the Import sheet, and the test of places runs over every tool ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)); [0004](0004-tools-for-people-operations-for-algorithms.md) counts Import as a tool, and [0083](0083-the-side-panels-layout-dragged-and-floating.md)'s layout takes only the Tools panel's tools.
- [0037](0037-one-view-the-photos-tab.md) and [0041](0041-dialogs-as-native-modal-windows.md) list Import and Print as full-window sheets; an alert a sheet asks is the main window's. Whether Print takes this frame is settled when Print is designed.
- The settings schema gains the sheet's own settings with owner `app`: `import.column_width`, `import.keep_open` and `import.thumbnails` (the grid's Zoom, Medium by default).
- `docs/gui.md` describes the sheet ([Sheets](../gui.md#sheets)); `docs/roadmap.md` holds the build order; the wireframe's Import sheet board draws it, and the `ui-conventions` skill is to describe the sheet.
