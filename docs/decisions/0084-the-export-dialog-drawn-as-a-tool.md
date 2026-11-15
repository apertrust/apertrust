# 0084. The Export dialog drawn as a tool

## Context

The Export dialog holds darktable's export module row for row ([0056](0056-export-as-an-engine-job.md)) in a native modal window, a sheet on the Mac ([0041](0041-dialogs-as-native-modal-windows.md)). Its settings are darktable's export settings, which darktable keeps in darktablerc and Apertrust in `settings.toml`, never in an edit: the same kind as a tool's own settings, which a tool panel already draws as `Row::Setting` ([0029](0029-a-tool-is-darktables-page.md), [0043](0043-settings-and-apertrusts-folders.md)). darktable draws its export module with the same widgets as its processing modules. The question is how the dialog's settings look and behave, how tall the dialog is, and what scrolls when it does not fit.

## Decision

**The Export dialog is a tool shown in a dialog.** Its settings column is drawn as a tool's page whose every row is a tool's setting, with the tool panel's components, sizes and rhythm, so a row looks and behaves in the dialog as it does in the side panel. Planned: the dialog built now is the earlier form, its choices Boxed ([gui.md](../gui.md#as-built)).

- **Every row is a tool setting's row** (`Row::Setting`'s look):
  - choices Blended, darktable's combobox as a tool draws it ([0042](0042-numbers-shown-typed-and-stored.md));
  - no kept marker on any row, sliders included, since nothing in the dialog is an edit; the slider's handle is unchanged;
  - no history step: a change is a setting written at once, as now; Undo in a field undoes typing only;
  - a slider's reset as a tool's: a double-click on its label or track, or Delete on it, puts back darktable's default (Quality 95); choices have no reset, as a tool's choices have none yet;
  - a row or an option not built drawn disabled, saying why, never hidden ([0030](0030-controls-as-darktables.md)).
- **Allow upscaling, High quality resampling and Store masks are toggles** (`ToggleRow`, the tool panel's checkbox row, 24 points, on the same edges as the rows round them), where darktable's export module has comboboxes of No and Yes: a deliberate difference from darktable, a boolean drawn as a tool draws one. Their settings are booleans already.
- **darktable's section labels stay section labels:** Storage options, Format options and Global options are drawn as a tool panel draws darktable's section labels (`group_label`, 8 points above and 2 below), not as collapsible groups. Metadata and Multi-preset export stay the only collapsible groups, as darktable has them.
- **No tool header in the column:** the dialog's title is its title, Default settings its reset, the Presets column its presets; there is no ⋯, since a dialog's settings cannot be moved, pinned or floated ([0083](0083-the-side-panels-layout-dragged-and-floating.md)).
- **Rows the tool panel lacks** are drawn in its language from its parts, each added to the design system with its gallery snapshots: a slider row with no kept marker, its place left empty so nothing moves; the path row, one 24-point line with the label, the path field stretching and the folder `IconButton` at the end; note rows for the session's lines, 12-point text on 16-point lines, `warning` and `error` with the warning sign; field rows, a label and a value field or a 180-point text field at the end; the size row, two or three value fields of 54 points with ×, the unit, @ and dpi, ending where slider values end; and an indented toggle, 20 points in.
- **Width:** 640 points, unchanged. The Presets column is 200, a separator 1 and the settings column 439, with the tool panel's 12-point sides, so its rows are 415 points wide and stretch as a tool's rows do in a widened side panel. Rows stack with no spacing between them, each carrying its own padding.
- **Height:** the settings column is 580 points by default, so every default row, Multi-preset export's included, fits without scrolling in English, Ukrainian and Polish on a first export; a second export in Polish scrolls by up to 13 points. With its 44-point header and 52-point footer the Mac's sheet is 676 points high; with the host's 32-point strip and the footer the window on Windows and Linux is 664.
  - **It shrinks with its room:** on the Mac to the main window's height less 74, as a sheet hangs inside its window; on Windows and Linux to the work area of the main window's monitor less 8. The settings column never goes below 430 points (the sheet 526, the window 514), which is what the least main window, 600 points high, leaves on the Mac at the interface's default scale. Where a larger interface scale on a small screen leaves less, the room wins: the column takes what is left, so the header and the footer stay inside the main window or the work area. A 13-inch Windows laptop at 1920 × 1080 and the recommended 150 %, 1280 × 720 points with its taskbar, has a 672-point work area, which sets the default.
  - **Only the settings column scrolls,** with the quiet scroll bar in its end margin, so rows never move when it appears. The header, the footer and the Presets column's bar stay in view; the presets list scrolls on its own.
  - **The height is fixed while the dialog is open:** a format with more rows, an opened group, a wrapped note or the hover preview's banner scrolls the column, and the window never jumps. A row reached by Tab or F6 scrolls into view whole. Before the session's first preview the file-name and size notes keep one line's height, so nothing moves when they arrive.
  - The interface scale multiplies all of it.
- **What stays the dialog's own:** the window and its forms ([0041](0041-dialogs-as-native-modal-windows.md), [0069](0069-dialogs-on-windows-and-linux-framed-by-the-host.md)), the title, the Presets column, the hover preview, the footer, Return, Escape, F6 and the alerts. The settings, commands and events are unchanged: a script sees the same `export.*` settings, `PreviewExport`, `Command::Export`, `CancelJob` and `RetryJob`.
- **Only Export changes now.** The other dialogs and panels, Settings and Customise tabs among them, keep Boxed choices until each is next changed, when whether it follows is decided.

## Options considered

- **The dialog's own form, as built:** Boxed choices, 4 points between rows, 16-point sides, the path on a line of its own. A second look for darktable's same widgets, and settings that read as a tool's in the side panel and as a form here.
- **Every dialog Blended at once:** one look everywhere, but Settings is not a tool, and whether it follows is better weighed once Export has been seen.
- **darktable's No and Yes choices for the three booleans,** the mirror rule: two clicks for what a tool draws as a toggle of one.
- **Collapsible groups for the three section labels:** darktable does not collapse them, a group is closed at first ([0029](0029-a-tool-is-darktables-page.md)) so Format options would hide Quality, and every export would cost a click.
- **A tool header in the column:** 30 points more, repeating the title, the reset and the presets the dialog already has.
- **A dialog 680 points wide:** every default note on one line in the three languages, but each label further from its value; 640 is the smaller change.
- **The side panel's default width for the rows, 296 points:** it cuts the default path, wraps every note in Ukrainian and Polish and leaves 120 points of the column empty.
- **A taller default:** the 672-point work area of a 1280 × 720 screen leaves no room past 580.
- **A dialog that grows with its content:** the window would jump as the format or a group changes.

## Consequences

- The design system gains the rows above, each in the gallery with snapshots; the slider row with no kept marker serves a tool's settings too, whose sliders draw one today.
- `crates/app/src/export.rs` is rebuilt from them, with Blended choices, the toggles, 12-point sides and no spacing between rows, and the height rule; the dialog's snapshots are taken again, and one at the least height.
- The dialog host tells the dialog its room, the main window's height on the Mac and the work area elsewhere, before its first, hidden frame.
- Decision 42's choice styles name the Export dialog with the tool panels; Boxed stays the default everywhere else.
- `docs/gui.md` describes the dialog ([The Export dialog](../gui.md#drawn-as-a-tool)), its deviation D15 and the choice styles; the wireframe's Export board is redrawn; the `ui-conventions` skill is to describe it.
