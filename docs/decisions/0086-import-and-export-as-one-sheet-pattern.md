# 0086. Import and Export as one sheet pattern

## Context

Import… and Export… both choose photos and hold settings beside them. darktable draws copy & import as a dialog listing the source's files with its options beside them (`src/libs/import.c`), and its export module in the lighttable's side panel, acting on the selected images (`src/libs/export.c`). Export was planned as a native modal dialog with a column of export tools ([0084](0084-export-as-a-sheet-of-export-tools.md), [0041](0041-dialogs-as-native-modal-windows.md)), and Import as a full-window sheet ([0037](0037-one-view-the-photos-tab.md)) with a column of import tools. Two forms for one shape of job teach two sets of gestures, and a dialog shows no photos to check. The questions are what a sheet is made of, how its photos are chosen, where its tools sit and what their headers hold, where presets live, what the footer says, its keys and modality, and which tools may appear where.

## Decision

**Import… and Export… open one kind of sheet:** full-window inside the main window, below the top panel, drawn with the design system's components on `panel`, the same on every platform, never one of AppKit's sheets. A sheet bar across its top, a grid of photos with check boxes, a column of tools beside the grid and a footer. The two differ only in their grid's content and marks, their tools, their notes and their footer's words ([0084](0084-export-as-a-sheet-of-export-tools.md) for Export's; Import's with its own decision). Planned: built in the order below.

### The regions

| Region | Where and how big | Holds |
| --- | --- | --- |
| **The sheet bar** | Across the sheet, as tall as the central bar, its 2-point separator under it; parted at the column's edge | Over the grid: the heading, the sheet's name in 16-point Medium then its source or photos and their count in `text_secondary`; the Select buttons; Sort; Zoom. Over the column: the presets row |
| **The grid** | The rest of the sheet's width, between the bar and the footer | The grid panel's `GridCell`s with a check-box caption |
| **The tools column** | 440 points by default, resizable at its edge from 296 to 600, and to no more than half the sheet where that leaves more than 296: 296 is the floor, below a 592-point sheet too; kept per sheet (`import.column_width`, `export.column_width`) | The sheet's tools, scrolling with the quiet scroll bar in its end margin |
| **The footer** | 52 points across the sheet, a hairline above | The summary on two lines at its start; Keep open, Close and the primary button at its end |

- **The column sits on the side panel's side,** the right today, the left when the side panel moves there, and the grid on the grid panel's: opening a sheet, thumbnails appear where thumbnails were and tools where tools were, and reading down the column ends above the primary button.
- **Narrow windows:** the column gives way only once the grid is down to two cells at its Zoom, then down to 296, and never below it: in the least window, 500 points wide, the column is 296 and the grid the 204 left, two Small cells; the Select buttons fold into one Select ▾ below about 520 points of grid; the summary's path is cut first. The bar and the footer always stay in view, and Close and the primary button are never cut.

### The grid and the checks

- **The cells** are the grid panel's: the picture fitted in its square, at half strength while unchecked; a caption always shown, the check box then the file name cut in its middle; the sheet's active photo's ring and no selected ring; at the picture's bottom right on the thumbnail scrim, a destination mark saying what happens at the destination (Import: imported before; Export: what On conflict does with a file there). Only the cells in sight are drawn.
- **The checks are what goes,** not a selection, so a stray click never drops a large choice. A click on a check box, or Space on a focused cell, turns its check on or off; a click on the picture makes it the sheet's active photo, whose notes the tools show, and leaves the checks; Ctrl-click (Cmd on a Mac) on a picture turns its check and makes it active; Shift-click on a picture checks the range from the active photo to it, and on a box, or Shift+Space, sets the run from the last cell touched as that one was set.
- **The sheet's active photo is its own:** it never changes the window's active photo or selection.
- **The bar's Select buttons,** quiet text buttons as darktable has them: Select all, Select none, and Select new (Import) or Select edited (Export). Import adds Checked only, a toggle that hides the unchecked cells.
- **The checks** are plain data (`Checks`: the checked set, the active photo, the last touched), unit-tested once for both sheets.

### The tools column

- **Each tool is a section with the tool panel's header,** 30 points: the chevron, the title, ↺ and ⋯; a click on it but on ↺ and ⋯ opens or closes the tool, its open state kept per sheet as a side panel section's is. No on switch and no peeking with H: nothing in a sheet edits a photo.
  - **↺** sets every setting the tool owns to darktable's default, those of a storage or a format not shown included.
  - **⋯** holds only Move Up and Move Down; a right-click on the header, Shift+F10 or the Menu key opens it. It is drawn disabled until reordering is built, its tooltip saying so.
- **The column reorders only:** by dragging a header, ⋯'s Move Up and Move Down, and Alt+↑ and Alt+↓ on a focused header, [0083](0083-the-side-panels-layout-dragged-and-floating.md)'s operations and announcements on a list of the sheet's own (`import.order`, `export.order`). No pinning, no floating and no other tab: the column is a short list beside a grid that is the work. Until reordering is built the order is the default.
- **Every row is a tool setting's row** (`Row::Setting`'s look): choices Blended, darktable's combobox as a tool draws it ([0042](0042-numbers-shown-typed-and-stored.md)); a boolean a toggle (`ToggleRow`), where darktable has a combobox of No and Yes; no kept marker on any row, sliders included; no history step, a change being a setting written at once, Undo in a field undoing typing only; a slider reset as a tool's, by a double-click on its label or track or Delete; a row or an option not built drawn disabled, saying why, never hidden ([0030](0030-controls-as-darktables.md)).
- **The rows the tool panel lacks** are drawn in its language from its parts, each added to the design system with gallery snapshots: a slider row with no kept marker, its place left empty; the path row, the label, `PathField` stretching with darktable's variable list and the folder `IconButton` at the end; note rows for what the session says, 12-point text on 16-point lines, `text_secondary`, then `warning` and `error` with the warning sign; field rows, a label and a value field or a 180-point text field at the end; the size row, two or three 54-point value fields with ×, the unit, @ and dpi; an indented toggle, 20 points in. A problem's fix is an `ActionRow` under its note, such as **Use $(MONTH)**.
- **A tool's states:** a tool not built has its header drawn disabled and closed, "Comes in a later version" in its tooltip; a problem in a row outlines its field, its note in `error`, and a closed tool with a problem opens and scrolls into view once, as the problem appears.

### Where a tool belongs

- **Every tool has one place,** declared with its definition, or beside its id where a section is drawn by its own code, as Browse and History are: the side panel (the tools that edit the photo and the utilities), Export (the export tools) or Import (the import tools).
- **Export and import tools never appear in the side panel.** Its dispatcher, the workspaces, built in or the person's, Customise tabs, a header's drag, ⋯'s Move to Tab and a drop on a tab's icon take only side-panel tools; a layout read from settings that names a tool of another place skips it, as [0083](0083-the-side-panels-layout-dragged-and-floating.md) skips an id the build does not know, and never places it as a new section.
- **Side-panel tools never appear in a sheet,** and each sheet's column draws only its own place's tools.
- **Ids are unique across the three places,** so Export's Metadata, Import's Metadata and the side panel's Metadata are three tools, their tooltips saying which.
- **A test** walks every tool and checks that each built-in workspace, Customise tabs' list and each sheet's default order hold only tools of their place, and that every layout operation given an export or import tool leaves the side panel unchanged.

### Presets: whole-sheet only

- **A sheet's presets hold all its tools' settings,** never the source (Import from's place and folder) or the checks. A tool has no presets of its own.
- **The presets row** sits in the column's part of the sheet bar, drawn as the side panel's preset row: Presets at its start, the matching preset's name or Custom at its end with its chevron, then ⋯ (Save as Preset…, Update, Rename…, Delete…, darktable's "delete preset?" and "update preset?" as alerts).
- **Its list,** drawn on every platform since it previews on hover: Default settings first, built in, setting every tool to darktable's defaults; then the sheet's built-in presets; then the person's, a ✓ on the one matching. Pointing at a preset shows its settings in the column, with a banner on the first tool, and the footer's summary follows, until the pointer leaves the list; ↑ and ↓ preview, Return or Space chooses.
- **Until presets are built,** the row is drawn disabled, showing Default settings, its tooltip saying presets come in a later version.

### The footer

- **The summary,** two lines at its start. The first, in `text`: the count, then the size or the format and size, then the destination cut in its middle ("209 of 412 photos · 6.2 GB → ~/Pictures/Apertrust/20261115_Wedding"). The second, smaller, says what may surprise before anything happens: in `warning`, photos imported before that will be copied again, or files that will be replaced; else why the primary button is disabled; else the sheet's reassurance. With nothing checked, "No photos checked".
- **Keep open** (`import.keep_open`, `export.keep_open`), a toggle before the buttons, off by default and kept, as darktable's import dialog has "keep this window open" by its buttons: with it on, the sheet stays after the job starts, and the second line confirms each start ("Started: 40 photos → …/client").
- **Close and the primary button** in the platform's order (`dialog_host::buttons`). Close, not Cancel: the settings were kept as they changed, and Cancel would read as cancelling a job a press started.
- **The primary button** ("Import 209 photos", "Export 12 photos") is disabled while nothing is checked, while Import's listing runs or while a problem stands; it stays focusable, its reason in its tooltip, in the footer's second line and as its accessible description.

### Keys, modality and screen readers

- **The sheet opens** with the keyboard focus on the grid's active cell and no focus ring until a key is pressed.
- **Return presses the primary button** wherever no text field is being typed in, no list is open and no button, choice or slider has the focus: a focused button, Close among them, presses itself, and a choice or slider opens its list or fine-control popup; a focused cell included; Ctrl+Return (Cmd+Return) from anywhere but an open list. Neither does anything while it is disabled.
- **Esc,** Cmd+. and Cmd+W close the innermost thing first (a list, the variable list, a field being typed in), then the sheet, keeping its settings and dropping its checks.
- **In the grid:** the arrows, Home and End move the focus and the sheet's active photo; Space checks; Shift+Space sets a run; Shift with the arrows checks the cells passed; Ctrl+A and Ctrl+Shift+A are Select all and Select none. **F6** and Shift+F6 move between three regions: the bar and the grid, the presets row and the column, the footer. The rows' keys are a tool panel's.
- **Modality:** while a sheet is open the view's keys and the top panel's actions rest, drawn at rest, and the menus that act on the view are disabled; the jobs spinner and View › Jobs stay live. One sheet at a time, and no dialog opens over one but the alerts it asks, which are the main window's. Closing the window closes the sheet first.
- **Screen readers:** the sheet is a region named by its heading under a dialog-role node; the grid a list named by its source and counts ("EOS_DIGITAL, 412 photos, 209 checked"), each cell named by its file, its check box "Import DSCF1033.RAF" or "Export IMG_2041.ARW", its mark in its description; the tools sections as in the side panel; a problem announced once, assertively, and the count politely, at most every 0.5 s.

### The build

1. **The setting rows,** in `apertrust-ui`, each in the gallery with snapshots.
2. **The sheet frame:** `Sheet` in `apertrust-ui` with its regions, the check-box caption and the mark slot on `GridCell`, `Checks`, the column of tools from definitions in an order list with the place and its test, the footer, the keys and the app's modality.
3. **Import's first slice** and **Export on the sheet,** each on the frame, in either order.
4. **The rest:** presets in both sheets, reordering, Import's later slices, Export's Metadata and Multi-preset export, a larger look at one photo.

The built Export dialog keeps working, as built, until Export is on the sheet.

## Options considered

- **Export a dialog beside Import's sheet,** a column of export tools in a native modal window: two forms for one job, a height rule for small screens, and no photos to check.
- **The column always at the window's start,** the left in a left-to-right language: the grid jumps to the other side from the grid panel as a sheet opens, and the column ends far from the primary button. Moving the side panel to the left gives that picture in every sheet.
- **Presets in each tool's ⋯ as well, or instead:** two places to look, a disabled ⋯ on every tool until presets come, and a naming scheme that spans Import to and Naming fits no single tool.
- **Presets as the column's first section,** its list always open: about 110 points of the column's height, scrolling away, a section that is not a tool among tools. **A Presets column** between the grid and the tools: the grid 200 points narrower.
- **Keep open as an import tool's row, and Export always closing:** one behaviour in two places, and no way to send several exports over the same checks.
- **Cancel to dismiss:** it reads as cancelling the job a press started; nothing in the sheet is undone.
- **The window's selection model for what goes:** a plain click selects one alone, dropping hundreds of chosen photos.
- **The column the side panel's width at opening:** at 320 points the default paths are cut and every note wraps in Ukrainian and Polish.
- **Return doing nothing on a focused cell,** kept for a larger look at one photo: the sheet opens on a cell, so the one-key import and export would be lost.
- **One of AppKit's sheets or a native window for each:** the grid needs the window's size, and a sheet drawn the same on every platform needs no host code.

## Consequences

- `apertrust-ui` gains the setting rows, `Sheet`, the check-box caption and mark slot on `GridCell`, `Checks`, and a preset row for the sheet bar; the app gains the sheets' modality.
- Tool definitions gain their place, and the test of places runs over every tool; [0004](0004-tools-for-people-operations-for-algorithms.md) counts a sheet's section as a tool, and [0083](0083-the-side-panels-layout-dragged-and-floating.md)'s layout takes only side-panel tools.
- [0037](0037-one-view-the-photos-tab.md) and [0041](0041-dialogs-as-native-modal-windows.md) list Export with Import and Print as full-window sheets; an alert a sheet asks is the main window's. Whether Print takes this frame is settled when Print is designed.
- The settings schema gains the sheets' own settings with owner `app`: `import.` and `export.` `column_width`, `order`, `keep_open` and `thumbnails` (the grid's Zoom, Medium by default).
- `docs/gui.md` describes the pattern ([Sheets](../gui.md#sheets)); `docs/roadmap.md` holds the build order; the wireframe gains a Sheets board, and the `ui-conventions` skill is to describe the pattern.
