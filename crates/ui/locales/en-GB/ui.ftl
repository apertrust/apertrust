# Apertrust's interface in British English, the source language (docs/gui.md, Translation).
# Every message id says where the text appears. Ukrainian (uk) and Polish (pl) translate
# every message here, with the same variables.

app-name = Apertrust

## The Toolbar's common actions

# Every action the Toolbar can show has a .toolbar attribute (decision 92), in every
# language: its word under its icon while Toolbar Labels is on, short and on one line, at
# most 96 points at the caption size, with no ellipsis. A tool's has it on its -full
# message (tools.ftl).
action-import = Import…
    .toolbar = Import
action-export = Export…
    .toolbar = Export
# File › Export Again: the export with the settings as last kept, with no dialog.
action-export-again = Export Again
    .toolbar = Export again
action-reset = Reset
    .toolbar = Reset
# Edit › Discard History… and the History section's options: clears the history, asking
# first.
action-discard-history = Discard History…
action-undo = Undo
    .toolbar = Undo
action-redo = Redo
    .toolbar = Redo
action-copy-edit = Copy edit
    .toolbar = Copy edit
action-apply-edit = Apply edit
    .toolbar = Apply edit
action-copy-apply-parts = Copy or apply parts
    .toolbar = Copy or apply parts
# The Select menu's items, darktable's select all, select none, invert selection and select
# untouched: they act on the Browser's photos, the folder open's.
action-select-all = Select all
action-select-none = Select none
action-invert-selection = Invert selection
action-select-unedited = Select unedited
# Goes back to the selection as it was before its last change, one level.
action-previous-selection = Previous selection
action-settings = Settings
    .toolbar = Settings
# Heard once the edit is saved again after a failed write (the Toolbar's save state).
status-saved = Saved

## The Tools panel's tabs

# Edit's tabs: the tools by editing stage, Adjust, Detail, Geometry, Effects and Raw,
# between Apertrust's own Photos and History (decision 38).
tab-photos = Photos
tab-adjust = Adjust
tab-detail = Detail
tab-geometry = Geometry
tab-effects = Effects
tab-raw = Raw
# darktable's four module groups' names, base, color, correct and effect (decision 38): no
# built-in workspace's tab takes them.
tab-base = Base
tab-color = Colour
tab-correct = Correct
tab-effect = Effect
tab-history = History
tab-metadata = Metadata
# The Hidden tab, last while Show Hidden Tools is on: the tools the workspace puts in no
# tab (decision 96).
tab-hidden = Hidden
tools-panel-no-hidden = No hidden tools in this workspace.
# Over the Tools panel's sections, once a tool moves in a workspace whose file a newer
# Apertrust wrote, which this build never overwrites (decision 96).
tools-panel-newer = A newer Apertrust saved this layout, so changes to it won't be kept
tools-panel-newer-tip = This version never overwrites a workspace file a newer Apertrust wrote. Tools you move stay where you put them until you quit.
# The dot on the Tools panel's ⋮, and Show Hidden Tools' tip: $tools names the hidden tools
# that change the photo, as "Exposure, Vignetting".
tools-panel-hidden-change = Hidden tools change this photo: { $tools }
# Find Tool… (decision 96): its field's name, which it shows while empty, and ✕; the hint
# under the empty field; what shows where nothing matches, $text what was typed; the count
# screen readers hear after each keystroke; and where a tool found lives, said after its
# title, $tab its tab's name.
find-tool-field = Find a tool
find-tool-close = Close search
find-tool-hint = Type a tool’s name.
find-tool-none = No tool matches “{ $text }”.
find-tool-count =
    { $count ->
        [one] { $count } tool
       *[other] { $count } tools
    }
find-tool-lives-in = in { $tab }
find-tool-lives-hidden = hidden
# The built-in workspaces' names in View › Workspace (decision 96); a person's own is
# named by its file.
workspace-default = Default
workspace-essentials = Essentials
# The tooltip of a tab drawn disabled because nothing is behind it yet; $tab is its name.
panel-tab-later = { $tab }: coming in a later version
tools-panel-tabs = Tools panel tabs
panel-more-tabs = More tabs
panel-tab-options = Tab options
panel-show-tab-names = Show tab names
panel-customise-tabs = Customise tabs…
tools-panel-width = Tools panel width
# The Tools panel's ⋮: its submenu of where the panel goes, after Workspace.
tools-panel-position = Position

## A section's header buttons; $section is the section's title

section-reset = Reset { $section }
section-presets = { $section } presets
section-options = { $section } options
# The tooltip of a tool's options button, drawn disabled while its menu holds nothing.
section-options-none = No options yet
# The tooltip of an item of a tool's options menu drawn disabled because it is not built yet.
section-options-later = Coming in a later version
section-toggle = Open or close { $section }. Hold H over it to see the photo without it.
# The name of a section's tab list, for screen readers.
section-tabs = { $section } tabs
# The tooltip of a tab drawn disabled because nothing is behind it yet.
section-tab-later = Coming in a later version
# The description screen readers hear on the section of a tool darktable leaves off on a
# new photo while it is off (decision 32): it renders nothing until a control changes.
section-not-applied = Not applied
# What screen readers hear as a tool moves in the workspace's layout (decisions 83 and 96):
# $tool is its name, $tab the tab's, $at its place among the tab's tools and $of how many
# the tab holds.
section-moved = { $tool } moved to { $at } of { $of } in { $tab }
section-pinned = { $tool } pinned in { $tab }
section-unpinned = { $tool } unpinned in { $tab }
section-moved-to = { $tool } moved to { $tab }
section-hidden = { $tool } hidden
section-shown = { $tool } shown in { $tab }
# A section header's tooltip (docs/gui.md, The header's tooltip): its rows' labels, and a
# row as screen readers hear it.
section-about-kind = Kind
section-about-works-on = Works on
section-about-measures = Measures
section-about-input = Input
section-about-process = Process
section-about-output = Output
section-about-row = { $label }: { $value }
# The tooltip of a choice's option drawn disabled, as one darktable lists that is not
# ported yet.
choice-option-later = Coming in a later version
# The tooltip and description of a choice's option an edit from another photo holds,
# which this photo does not list (decision 30).
choice-option-unavailable = Not available for this photo

## A collapsible group inside a tool, darktable's collapsible section

# What screen readers hear after its title while controls in it are off their defaults;
# $count is how many.
collapsible-group-set = { $count } set
# The tooltip of a closed group while controls in it are set; $group is its title and
# $count how many.
collapsible-group-tip = { $group }: { $count } set
# A collapsible group's title while its settings make what it holds act, as darktable's
# "area color mapping (active)": $group its title.
collapsible-group-active = { $group } (active)

## A preset row in a tool, whose options each write several of its controls, as White
## balance's Mode

# What its box shows while the controls hold none of its options' values; never an option
# itself.
preset-custom = Custom
# The tooltip of an option drawn disabled because this photo cannot have it.
preset-option-unavailable = Not available for this photo

## A slider's marker

slider-kept = Kept: you set it, so Auto and Match leave it. Click to release it.
slider-keep = Click to keep this value.
# The marker's name for screen readers; $control is the slider's full name.
marker-keep = Keep { $control }
# A value field's name for screen readers.
field-value = { $control }: exact value
# A slider's value field's name, the field holding the number alone; $unit is the unit as
# unit-ev writes it.
field-value-in = { $control }, in { $unit }
# The Match tick's tooltip; $value is the slider's value before Match.
slider-before-match = Before Match: { $value }
# The tooltip of a slider's value, shown with its unit, which a click makes a field; on
# the Mac the key is Return, as its keyboard names it.
slider-value-tip = Click, or press Enter on the slider, to type a value.
slider-value-tip-mac = Click, or press Return on the slider, to type a value.

## Themes

theme-dark = Dark
theme-grey = Grey
theme-light = Light
theme-high-contrast = High contrast

## Counts

photo-count =
    { $count ->
        [one] { $count } photo
       *[other] { $count } photos
    }

## Units, after a value

unit-ev = EV
unit-percent = %
unit-degrees = °
unit-kelvin = K
unit-pixels = px
unit-iso = ISO
unit-mib = MiB
unit-gb = GB

## A value with its unit, as a slider shows it and screen readers hear it; $value is
## already in the language's number format, signed where the slider's range spans zero

unit-ev-value = { $value } EV
unit-percent-value = { $value }%
unit-degrees-value = { $value }°
unit-kelvin-value = { $value } K
unit-pixels-value = { $value } px
unit-iso-value = { $value } ISO

## The Toolbar

toolbar = Toolbar
# The menu button at the Toolbar's start on Windows and Linux, the app's icon or ☰, which opens
# the menus.
toolbar-menu = Menu
# », holding the actions a narrow Toolbar has no room for.
toolbar-more = More actions
# The toolbar menu, on a right-click anywhere on the Toolbar, and View › Panels (decision
# 92): the words under the icons on or off, and the Customise Toolbar window, drawn
# disabled with its tip until it is built.
action-toolbar-labels = Toolbar labels
action-customise-toolbar = Customise toolbar…
# Hide "Undo", on the item the menu opened on, $item its name: it leaves the workspace's
# Toolbar. Show Menu Button puts the menu button back while it is hidden.
action-toolbar-hide = Hide toolbar item
toolbar-menu-hide = Hide “{ $item }”
# What screen readers hear once Hide has taken the item away.
toolbar-item-hidden = { $item } hidden
action-show-menu-button = Show menu button
toolbar-customise-later = Customise Toolbar comes in a later version
# The save state at the Toolbar's end, which shows only when a save goes wrong:
# Saving… while a write is still unconfirmed about two seconds after it started, as on a
# slow network folder.
status-saving = Saving…
status-saving-tip = Still writing this photo’s edit beside it
# A button in the warning colour while the last write failed; $error is why, in the
# system's words: its tooltip, and what screen readers hear as it fails.
status-not-saved = Not saved
status-not-saved-reason = Not saved: { $error }
# In the caption, while the photo open is open for viewing only: a newer Apertrust
# wrote its edit, which this one never saves.
status-view-only = View only
# The window's caption buttons at the end of the Toolbar, where the app draws them.
window-minimise = Minimise
window-maximise = Maximise
window-restore = Restore
window-close = Close

## The jobs: the spinner at the Toolbar's end and the window it opens. A row's lines and
## buttons are the activity's words below. $running, $waiting and $failed count jobs, $value
## is the spinner's value and $message a job's own message. The spinner's value avoids
## plural forms: "2 running".

jobs-name = Jobs
# View › Jobs: the jobs window, opened or brought to the front.
action-show-jobs = Jobs
    .toolbar = Jobs
jobs-tip = Jobs: { $value }
jobs-value-running = { $running } running
jobs-value-waiting = { $waiting } waiting
jobs-value-running-waiting = { $running } running, { $waiting } waiting
jobs-value-failed = { $failed } failed
jobs-empty = No jobs.
jobs-queued = Waiting for other jobs
# An ended row's title where the job gives no outcome, which the live region also says.
jobs-finished = Finished: { $message }
jobs-failed = Failed: { $message }
jobs-cancelled = Cancelled: { $message }
# The ✕ at the end of a row's bar.
jobs-cancel = Cancel
jobs-cancel-tip = Stop this job
# The footer's button, which takes away every finished and cancelled row.
jobs-clear-finished = Clear Finished
# A failed row's menu item, which copies its failure as the row says it.
jobs-copy-error = Copy Error
# What the live region says as a job starts.
jobs-live-started = Started: { $message }

## The jobs' own messages, as darktable words its jobs

job-import = Importing { $done } / { $total }
job-delete = Deleting { $done } / { $total }
job-thumbnails = Creating thumbnails
# A folder being read as it opens; $folder is its path.
job-folder = Reading { $folder }

## The cache's Clear: $kind is what it clears, thumbnails or pipelines; $done the entries
## done of $total; $entries those it removed, $size and $unit how much they held, the unit
## kb, mb or gb.

job-cache-size =
    { $size } { $unit ->
        [kb] kB
        [gb] GB
       *[mb] MB
    }
job-cache-entries =
    { $kind ->
        [thumbnails]
            { $entries ->
                [one] { $entries } thumbnail
               *[other] { $entries } thumbnails
            }
       *[pipelines]
            { $entries ->
                [one] { $entries } pipeline
               *[other] { $entries } pipelines
            }
    }
# Its title, and the line under its bar.
job-cache-clear =
    { $kind ->
        [thumbnails] Clearing the thumbnail cache
       *[pipelines] Clearing the pipeline cache
    }
job-cache-clear-now = { $done } of { $total } removed
# Its title once it has ended: "Cleared 2,104 thumbnails, 138 MB".
job-cache-cleared = Cleared { job-cache-entries }, { job-cache-size }
job-cache-clear-cancelled = Clearing cancelled: { job-cache-entries } removed, { job-cache-size }

## What an export job reports of itself for its row. $number is the photo under way,
## from 1, and 0 before the first; $total the photos; $storage where they go, "disk" for
## File on disk; $photo a raw; $done the photos done; $count the photos waiting; $folder
## where they go, as typed; $written, $skipped and $failed the photos written, left as On
## conflict says, and failed. A single photo is the exact number 1, never the plural
## category one, which Ukrainian also gives 21.

# Its title, darktable's message.
job-export-photo =
    Exporting { $number } / { $total } to { $storage ->
       *[disk] file on disk
    }
# The line under its bar while it waits, while a photo is under way and between photos.
job-export-queued = Next in line: { photo-count }, to { $folder }
job-export-now =
    { $total ->
        [1] Exporting { $photo }
       *[other] { $done } of { $total } done · now { $photo }
    }
job-export-between = { $done } of { $total } done
# Its title once it has ended, which says how it went.
job-export-finished =
    { $skipped ->
        [0]
            { $written ->
                [1] 1 photo exported
               *[other] { $written } photos exported
            }
       *[other]
            { $total ->
                [1] The photo was skipped
               *[other] { $written } of { $total } photos exported, { $skipped } skipped
            }
    }
job-export-failed-outcome =
    { $total ->
        [1] The photo could not be exported
       *[other] Export failed: { $written } of { $total } photos exported
    }
job-export-cancelled =
    { $total ->
        [1] Export cancelled
       *[other] Export cancelled: { $written } of { $total } photos exported
    }

## Why a job failed, in words, without a full stop: a code the engine sends. They end a
## line such as "IMG_2047.ARW: could not write IMG_2047.jpg: the disk is full."

job-cause-unreadable = the raw file cannot be read
job-cause-edit = its edit cannot be read
job-cause-binding = an edited tool cannot be applied to the photo
job-cause-size = the size asked for does not fit the format
job-cause-pattern = the file name is not valid
job-cause-name = the file name gives this photo no name
job-cause-render = the photo could not be rendered
job-cause-encode = the file could not be encoded
job-cause-disk-full = the disk is full
job-cause-read-only = the disk or folder is read-only
job-cause-gone = the folder, or its disk, is no longer there
job-cause-folder = the folder cannot be made or written to
job-cause-write = the file could not be written
job-cause-engine = the engine stopped unexpectedly
job-cause-cache-remove = some of the cache's files could not be removed
job-cause-cache-read = the cache's folder cannot be read

## A job's row in the jobs window: its lines and buttons. $folder is where its results go,
## $time when it ended on the local clock ("14:02"), $photo the item it failed on, $file the
## file it could not write, and $cause why, as the app words it.

# A row's lines.
activity-folder = To { $folder }
activity-folder-at = To { $folder } · { $time }
activity-failure = { $photo }: could not write { $file }: { $cause }.
activity-failure-photo = { $photo }: { $cause }.
activity-failure-cause = { $cause }.
# A row's buttons: Show names the platform's file manager.
activity-show-finder = Show in Finder
activity-show-explorer = Show in Explorer
activity-show-files = Show in Files
activity-show-tip = Open the export folder
activity-try-again = Try again
activity-try-again-tip = Export the photos that failed again, with the same settings
activity-dismiss = Dismiss

## The viewer bar, over the photo

viewer-bar = Viewer bar
photo-show-selected = Show all selected photos
photo-proof-margin = Proof margin
photo-before-after = Before and after
guides-show-label = Show guides
guides-show-full = Show guides
guides-show-promise = Show the guide overlay while this tool has focus.
guides-settings-full = Change global guide settings
guides-settings-promise = These settings apply globally, to every tool that shows guide overlays.
photo-layer = Layer: { $layer }. Change layer
photo-new-layer = New layer
photo-values = Values under the cursor
photo-values-read = Red { $red }, green { $green }, blue { $blue }, grey { $grey }
photo-zoom = Zoom
photo-zoom-small = Small
photo-zoom-fit = Fit
photo-zoom-fill = Fill
photo-zoom-in = Zoom in
photo-zoom-out = Zoom out
# The zoom's tooltip. $command is the pointer's modifier as shortcuts spell it: "Cmd" on
# the Mac, "Ctrl" elsewhere.
photo-zoom-tip = Zoom: pinch or { $command }+scroll; Z for 100%; middle-click cycles 100%, 200% and Fit. Scroll moves a zoomed photo, Shift+scroll sideways
photo-zoom-tip-exact = At 100% the screen shows exactly what a full-size export will be. Zoom: pinch or { $command }+scroll; Z for 100%; middle-click cycles 100%, 200% and Fit. Scroll moves a zoomed photo, Shift+scroll sideways
# The photo as a focus stop while zoomed past Fit: its name, and its value, the scale and
# the part shown in whole percentages.
photo-view = Photo, { $name }
photo-view-value = { $scale }, { $left } to { $right }% across, { $top } to { $bottom }% down
layer-image = Image

## The Viewer

# While the photo decodes and its first frame renders; $name is its file name.
photo-opening = Opening { $name }…
# The viewer bar, quietly, while a photo opening is still not live a second after the
# click (decision 77).
photo-rendering = Rendering…
# The tooltip of every control while the photo opens, read-only until it is live; $name
# is the photo's file name without its extension.
controls-opening = Opening { $name }…
# Over the photos opened after one whose edit could not be written as it closed, so that
# edit is lost: $name is that photo's file name, $error why, in the system's words.
edit-not-saved = The edit of { $name } was not saved: { $error }
# In the tooltip of the caption under the photo, after its camera and size, when a newer
# Apertrust wrote its edit: this one would lose what it does not know by saving it, so the
# photo opens for viewing only and its controls are disabled. $version is the version that wrote
# the edit, as the edit records it ("0.3.0", "0.2.0+14~g1a2b3c4").
photo-view-only = Edited in a newer Apertrust, { $version }: open for viewing only
# The same where the edit does not say which version wrote it.
photo-view-only-unknown = Edited in a newer Apertrust: open for viewing only
# In the same tooltip when part of the edit's history cannot be read, as a damaged or hand-edited
# file's: saving would change what could not be read, so the photo opens for viewing only.
photo-view-only-unreadable = Some of this edit’s history can’t be read: open for viewing only
# The same where part of the history holds what this build would drop by saving it, as a
# newer build's field.
photo-view-only-unkept = Some of this edit’s history can’t be kept: open for viewing only
# Why an edit was not saved, after "The edit of … was not saved:" and in the save state's
# tooltip: another program changed its file, which this one can't read, so it is not written
# over.
edit-not-saved-unreadable = another program changed its file, and this Apertrust can’t read it
# The same where the file another program wrote opens for viewing only now.
edit-not-saved-view-only = another program changed its file, which opens for viewing only now
# The same where another program kept changing it while the photo closed, each write finding
# it changed again.
edit-not-saved-still-changing = another program kept changing its file as the photo closed
# Over the photo, until another is opened, when another program changed the history in its
# edit's file so that the last Undo, Redo, Go to, Compress, Truncate or Discard History made
# here could not be kept. $name is the photo's file name.
edit-move-not-kept = Another program changed the history of { $name }: your last Undo, Redo, Go to, Compress, Truncate or Discard History wasn’t kept
# The exposure values at the end of the caption under the photo, from its EXIF. $value is
# written in the language's numbers: the shutter as "1/250" or "2.5", the aperture "8.0",
# the focal length "35", the ISO "200".
photo-shutter = { $value } s
photo-aperture = f/{ $value }
photo-focal-length = { $value } mm
photo-iso = ISO { $value }

## The platform's open dialog, from File › Open…

# Its title, where the platform shows one.
open-title = Open
# The file types it shows: the raws Apertrust can open.
open-filter = Raw photos

## The Browser and its cells

browser = Browser
# A cell's mark and its tooltip, for a photo whose edit exists. A cell is named by its
# file name; its marks follow in its description.
grid-cell-edited = Edited
# The status mark of a photo whose edit a newer Apertrust wrote: it opens for viewing only.
grid-cell-view-only = View only: a newer Apertrust wrote its edit
# The status mark of a photo whose sidecar could not be read; $reason is the library's.
grid-cell-unreadable = Its edit could not be read: { $reason }
# The caption under a thumbnail, for screen readers: the photo's rating ($count stars, 0
# to 5), its reject and its colour labels ($labels, their names in order).
grid-cell-rating =
    { $count ->
        [0] No stars
        [one] { $count } star
       *[other] { $count } stars
    }
grid-cell-rejected = Rejected
grid-cell-labels = Colour labels: { $labels }
# The caption's first line: a star that rates the photo $count stars, the reject mark,
# and the circle that lists the colour labels, its value with none set.
grid-cell-rate =
    { $count ->
        [one] Rate { $count } star
       *[other] Rate { $count } stars
    }
grid-cell-reject = Reject
grid-cell-colour-labels = Colour labels
grid-cell-no-labels = None
grid-cell-clear-labels = Clear labels
# What screen readers hear first in a cell's description, after its file name, for a
# photo whose file holds no picture Apertrust reads and that has not been rendered yet:
# its cell shows an image glyph on a plain plate in place of a picture.
grid-cell-no-preview = No preview yet
# The line the same cell's tooltip ends with: Apertrust renders a preview of the photo
# once it is opened.
grid-cell-no-preview-tip = No preview in this file yet. Apertrust makes one when you open it.
# darktable's colour labels, in its order.
colour-label-red = Red
colour-label-yellow = Yellow
colour-label-green = Green
colour-label-blue = Blue
colour-label-purple = Purple
# The Browser's edge towards the photo, which resizes it, for screen readers, and its
# value: how many rows of thumbnails it shows.
browser-height = Browser height
grid-rows =
    { $count ->
        [one] { $count } row
       *[other] { $count } rows
    }
# At the left or right, the Browser's edge resizes it in columns.
browser-width = Browser width
grid-columns =
    { $count ->
        [one] { $count } column
       *[other] { $count } columns
    }
# The browser bar's options menu, a right-click on it, for screen readers, and its submenu of
# where the Browser goes.
browser-bar-options = Browser options
browser-position = Position
# The browser bar, the header along the Browser's top, as screen readers name it.
browser-bar = Browser bar
# Its Sort button, its value for screen readers and its tooltip: the order, by $key
# (capture-time, file-name, edited or rating) and $descending (yes or no).
grid-sort = Sort
grid-sort-value =
    { $key ->
        [capture-time] Capture time
        [file-name] File name
        [edited] Edited
       *[rating] Rating
    }, { $descending ->
        [yes] descending
       *[no] ascending
    }
grid-sort-tip =
    Sort: { $key ->
        [capture-time] capture time
        [file-name] file name
        [edited] edited
       *[rating] rating
    }, { $descending ->
        [yes] descending
       *[no] ascending
    }
# Its Zoom button, the thumbnails' size, by $size (small, medium, large or extra-large).
grid-thumbnail-size = Thumbnail size
grid-thumbnail-size-value =
    { $size ->
        [small] Small
        [medium] Medium
        [large] Large
       *[extra-large] Extra large
    }
grid-thumbnail-size-tip =
    Thumbnail size: { $size ->
        [small] small
        [medium] medium
        [large] large
       *[extra-large] extra large
    }
# The browser bar's count, between Sort and Zoom: how many photos are selected, $selected,
# of the $total the Browser shows, "3/120", or the total alone with none selected, "120".
browser-count =
    { $selected ->
        [0] { $total }
       *[other] { $selected }/{ $total }
    }
# In the column at the Browser's left end, a number over 99,999 in thousands, $thousands:
# a letter or two, as the column is 36 points wide.
browser-count-thousands = { $thousands }k
# The count's tooltip, and what screen readers hear of it: with photos selected, and with
# none.
browser-selected-count =
    { $total ->
        [one] { $selected } of { $total } photo selected
       *[other] { $selected } of { $total } photos selected
    }
browser-photo-count =
    { $total ->
        [one] { $total } photo
       *[other] { $total } photos
    }

## Open kinds: what a window holds, named wherever a recent or what is open is

# One of a kind by its name: Open Recent's items, and Collections' top line and its recents.
# A library's joins it here, open-kind-library, with the library, as start-kind-library
# joins the start window's.
open-kind-folder = Folder: { $name }

## Folder mode: File › Open Folder…, the Photos tab's Collections and the empty states

action-open-folder = Open Folder…
    .toolbar = Open folder
# The title of the platform's folder dialog, where it shows one.
open-folder-title = Open Folder
# The Photos tab's first section, which chooses what the Browser shows, and its group
# of folders. Albums and Smart albums join it with the library.
section-collections = Collections
    .search = collections
collections-folders = Folders
section-export = Export
    .search = export
# The top line's box: what screen readers call it, and its words for none, which the
# Viewer says too with nothing open; a folder's are its open kind's.
collections-open-name = What is open
collections-no-folder = No folder open
# The box's list, the recents: with none; a missing one, marked, its label its open kind's
# ("Folder: 2026-10 Lisbon"), and what it offers.
collections-recents-none = No recent folders
collections-recent-missing = { $label }, not found
collections-try-again = Try Again
collections-locate = Locate…
collections-remove = Remove from the List
# The title of the platform's folder dialog Locate… opens.
collections-locate-title = Locate Folder
# ＋ beside the box, and its items that come with the library.
collections-plus = New or open
action-new-library = New Library…
# The tree: a folder row's right-click menu, whose Show in Folder is named as the platform's
# file manager is, and whose Open chooses the folder.
collections-folder-menu = { $name } menu
collections-open-row = Open
# A photo opened alone: Folders' one row and Collections' option, which open its folder in
# folder mode, the photo active.
collections-open-this-folder = Open This Folder
# Under a folder in the tree whose subfolders are being listed.
collections-reading = Reading…
# Under a folder in the tree whose subfolders could not be listed; the reason is in its tooltip.
collections-unreadable = Could not be read. Try Again
# The top line while the open folder is read.
folders-reading = Reading { $name }…
# The tooltip of a recent folder that could not be opened when it was tried.
folders-recent-missing = This folder could not be opened. { $reason }
# The tooltip of a recent folder that is no longer where it was.
folders-recent-gone = This folder is no longer there.
folder-reading = Reading { $folder }…
folder-empty = No photos available
# Under it in the Browser, where the folder has subfolders.
folder-empty-hint = Choose a subfolder in Collections.
folder-failed = Could not open { $folder }: { $reason }
folder-cancelled = Reading { $folder } was cancelled
# A folder is open with photos, and none is.
folder-no-photo = No photo selected

## The start window, which shows in place of the main window with nothing open

# Its list's heading, and what screen readers call the list.
start-recents = Recents
# The list with nothing in it.
start-recents-none = Nothing opened yet
# A recent's kind, at its row's end.
start-kind-folder = Folder
# Before the path of a recent that is no longer there, or that could not be opened.
start-recent-gone = Not found
start-recent-failed = Could not be opened
# What screen readers say of a recent after its name: its kind and its path, and why it
# cannot be opened where it cannot.
start-recent-about = { $kind }, { $path }
start-recent-about-missing = { $kind }, { $path }. { $why }
# The accent button, which opens the recent chosen.
start-open = Open
# The column at its start: the copyright under the version, and what the app is, on
# two lines.
start-copyright = © 2026
start-tagline =
    A free, non-destructive
    raw photo editor
# Over the window's top after a start that did not finish with a graphics card or API
# (decision 82). $card is a graphics card's name, or CPU's word, and $api a graphics API's,
# never translated; a message leaves out the one that was Automatic.
start-failed = { app-name } did not start last time with { $card } on { $api }, so it started with Automatic
start-failed-card = { app-name } did not start last time with { $card }, so it started with Automatic
start-failed-api = { app-name } did not start last time on { $api }, so it started with Automatic
# The same, where the graphics API it failed with was left out: $other is the API it
# started with, never translated.
start-failed-on = { app-name } did not start last time with { $card } on { $api }, so it started with Automatic on { $other }
start-failed-api-on = { app-name } did not start last time on { $api }, so it started with Automatic on { $other }
# After a start with Automatic that did not finish.
start-failed-automatic = { app-name } did not start last time with Automatic, so it started on the CPU, which is slower
start-failed-automatic-again = { app-name } did not start last time with Automatic, and this graphics API has no CPU adapter, so it started with Automatic again
# Its buttons: the settings set to what runs, or the app restarted with them.
start-failed-keep-automatic = Keep Automatic
start-failed-keep-automatic-on = Keep Automatic on { $api }
start-failed-keep-cpu = Keep CPU
start-failed-retry = Try again
start-failed-close = Close
# Under it, after Keep, where the command line sets the graphics card or API for this run,
# which then can't change. $flag is the flag, --conf, never translated.
start-failed-overridden = { $flag } sets the graphics card or API for this run, so nothing was kept. Start { app-name } without it to keep this.

## Settings

settings-close = Close
# The field at the top of the Settings dialog's page list, its placeholder and its name for
# screen readers: it finds a setting by its label or its help.
settings-find = Find a setting
# Why a row is dimmed, its tooltip: the command line set it for this run (decision 43).
# $flag is the flag, --conf, never translated.
settings-overridden = Set by { $flag } for this run
# Why a row is dimmed, its tooltip: the switch it needs is off, as Cache limit while the
# cache is unlimited. $setting is that switch's row's label.
settings-requires = Off while “{ $setting }” is off
# What a row's label tells screen readers while its changed mark shows: the setting is
# off its default.
settings-changed = Changed from its default
# A switch's default as a row's changed mark names it: "Reset to On".
settings-default-on = On
settings-default-off = Off
# A text's default, where it is no text at all: "Reset to empty".
settings-default-empty = empty
# The item of a row's right-click menu that puts its default back.
settings-reset = Reset
# The tooltip and screen-reader name of a row's changed mark, ●, a button that puts the
# default back. $value is the default as the row writes it: an option ("Dark"), a number
# with its unit ("100 %"), On or Off.
settings-reset-to = Reset to { $value }
# After a row's label, behind a middle dot, where its setting takes effect only at the
# next start: "Memory budget · after a restart".
settings-restart-note = after a restart
# A settings value with its unit, as a slider or a number box writes it: "110 %",
# "2048 MiB". $value is the number or a range in words, $unit the unit's words.
settings-value-with-unit = { $value } { $unit }
# Under a number box whose typed value is out of its range: what it may be. The unit
# follows through settings-value-with-unit.
settings-number-range = { $min } to { $max }
# The same, for a number with no maximum.
settings-number-range-from = { $min } or more
# The same, for a number with no minimum.
settings-number-range-up-to = up to { $max }
# The same, for a number that must be more than a value (the schema's `above`), with no
# maximum.
settings-number-range-above = more than { $min }
# The same, for a number that must be more than a value, with a maximum.
settings-number-range-above-to = more than { $min }, up to { $max }
# Screen-reader names of a number box's − and + buttons. $control is the row's label.
settings-number-decrease = Decrease { $control }
settings-number-increase = Increase { $control }
# The pages, listed at the dialog's start, and what a page that holds nothing yet says.
settings-page-general = General
settings-page-import = Import
settings-page-photos = Photos
settings-page-editing = Editing
settings-page-processing = Processing
settings-page-storage = Storage
settings-page-shortcuts = Shortcuts
settings-page-presets = Presets
settings-page-ai = AI
settings-page-later = Coming in a later version
# The titled groups of a page's rows.
settings-group-interface = Interface
settings-group-start = Start
settings-group-confirmations = Confirmations
settings-group-thumbnails = Thumbnails
settings-group-tools-panel = Tools panel
settings-group-hardware-acceleration = Hardware acceleration
settings-group-diagnostics = Diagnostics
settings-group-cache = Cache
# Each setting's row, by the setting's id, its dots and underscores as hyphens; a row's
# -help, where it has one, is its label's tooltip.
setting-theme-label = Theme
setting-theme-option-dark = Dark
setting-theme-option-grey = Grey
setting-theme-option-light = Light
setting-theme-option-high-contrast = High contrast
setting-locale-label = Language
# The language's default, which the app works out from the system's: its changed mark says
# "Reset to" these words.
setting-locale-default = the system's language
setting-scale-label = Interface size
setting-sliders-marker-label = Slider marker
setting-sliders-marker-option-triangle = Triangle
setting-sliders-marker-option-circle = Circle
setting-sliders-marker-option-square = Square
setting-toolbar-menu-button-label = Menu button
setting-toolbar-menu-button-option-icon = { app-name } icon
setting-toolbar-menu-button-option-menu = Menu
setting-toolbar-labels-label = Toolbar labels
setting-library-reopen-last-label = Reopen what was open last
setting-library-reopen-last-help = Opens the folder you were working in when { app-name } closed, with its photo. A file or folder opened from the file manager or a terminal opens instead.
setting-export-ask-before-overwrite-label = Ask before overwriting an exported file
setting-history-ask-before-discard-label = Ask before discarding a photo’s history
setting-browser-square-thumbnails-label = Square thumbnails
setting-library-raw-thumbnails-from-label = Raw thumbnails from
# Raw thumbnails from's tooltip: the setting applies to photos with no edit; those with an
# edit, or shown in the app for at least 300 ms, show its own render.
setting-library-raw-thumbnails-from-help = For photos with no edit: thumbnails larger than this size are rendered from the raw, in { app-name }'s look, instead of the camera's embedded JPEG. Slower. Photos with an edit, or shown in { app-name } for at least 300 ms, always show { app-name }'s render.
setting-library-raw-thumbnails-from-option-always = Always
setting-library-raw-thumbnails-from-option-small = Small
setting-library-raw-thumbnails-from-option-VGA = VGA
setting-library-raw-thumbnails-from-option-720p = 720p
setting-library-raw-thumbnails-from-option-1080p = 1080p
setting-library-raw-thumbnails-from-option-WQXGA = WQXGA
setting-library-raw-thumbnails-from-option-4K = 4K
setting-library-raw-thumbnails-from-option-5K = 5K
setting-library-raw-thumbnails-from-option-6K = 6K
setting-library-raw-thumbnails-from-option-8K = 8K
setting-library-raw-thumbnails-from-option-never = Never
setting-library-raw-thumbnails-from-option-auto = Auto
setting-tools-panel-tab-names-label = Show tab names
setting-tools-panel-wheel-scrolls-panel-label = Mouse wheel scrolls the Tools panel
setting-tools-panel-wheel-scrolls-panel-help = On, the wheel changes a slider or a choice under the pointer only with Ctrl held, Option on the Mac. Off, as in darktable, it changes one plain, a slider by a tenth of a step with that key held.
setting-engine-adapter-label = Graphics card
setting-engine-backend-label = Graphics API
setting-engine-backend-option-auto = Automatic
setting-engine-backend-option-dx12 = Direct3D 12
setting-engine-backend-option-vulkan = Vulkan
setting-engine-memory-budget-label = Memory budget
# Processing's Hardware acceleration group (decision 82). The graphics card's list:
# Automatic first; the In use line says what it runs. $card is an adapter's name and $api
# a graphics API's, never translated.
settings-gpu-automatic = Automatic
# The software adapter, whatever the platform calls it: the processor renders.
settings-gpu-cpu = CPU (no graphics card), slow
# The software adapter where a sentence names it.
settings-gpu-cpu-short = CPU
# Why CPU cannot be chosen: the graphics API has no software adapter.
settings-gpu-cpu-missing = Not found with { $api }
# Where the API Automatic stands for is not known yet.
settings-gpu-cpu-missing-unnamed = Not found with this graphics API
# Under the row, in the warning colour, while the card kept is not on this computer.
settings-gpu-not-found = Graphics card not found: Automatic used
# The card kept, listed but not to be chosen.
settings-gpu-not-found-item = Not found on this computer
# Under the row, in the warning colour, after a change of the graphics API left the card
# out.
settings-gpu-dropped = { $card } isn’t available with { $api }: set to Automatic
settings-gpu-dropped-unnamed = { $card } isn’t available with this graphics API: set to Automatic
# Under the row while the cards are listed, after a moment.
settings-gpu-listing = Looking for graphics cards…
# Under the row, in the warning colour, where listing them failed.
settings-gpu-not-listed = The graphics cards could not be listed
# Under the graphics API's row, in the warning colour.
settings-gpu-no-adapter = No graphics card found with { $api }: Automatic used
settings-gpu-no-adapter-unnamed = No graphics card found: Automatic used
# The line under the group's rows: the adapter and the graphics API running now.
settings-gpu-in-use = In use
settings-gpu-in-use-value = { $card }, { $api }
# Graphics APIs' names, never translated; Direct3D 12 and Vulkan are the row's options.
settings-gpu-api-metal = Metal
settings-gpu-api-gl = OpenGL
# Under the In use line, in the warning colour, with no graphics card in use.
settings-gpu-software = No graphics card in use: { app-name } renders on the processor, which is slower
# After it, where Automatic fell to the processor rather than the person choosing it.
settings-gpu-driver = Updating the graphics driver may help
# Under the In use line while a setting that applies after a restart differs from the
# value this run started with, then its button and the button's tooltip.
settings-restart-waiting = Your choice applies when { app-name } restarts
settings-restart-now = Restart now
settings-restart-tip = Quits and opens { app-name } again, with this folder and photo
# Under it, in the warning colour, where the app to start again could not be found; $error
# is the system's, untranslated.
settings-restart-failed = { app-name } can’t be started again: { $error }
setting-cache-limited-label = Limit the cache's size
setting-cache-limited-help = Off, the cache is unlimited: nothing is removed from it to keep it within a size.
setting-cache-limit-label = Cache limit
setting-cache-limit-help = Past this, the thumbnails used longest ago are removed, and made again when needed.
setting-cache-unused-after-label = Library caches unused for
setting-cache-unused-after-help = A library's cache not opened on this computer for this long is offered for removal. Nothing is removed without you.
setting-cache-unused-after-option-days_30 = 30 days
setting-cache-unused-after-option-days_90 = 90 days
setting-cache-unused-after-option-days_180 = 180 days
setting-cache-unused-after-option-days_365 = 365 days
setting-cache-unused-after-option-never = Never
setting-log-keep-label = Keep a log of this session
setting-log-keep-help = Keeps this session's log in memory from the moment it is on, for Help › Show Log. To log from the start, restart Apertrust. Nothing is written to a file unless you save it.
setting-log-domains-label = Log
setting-log-domains-help = What the log keeps besides warnings and errors, which it always keeps. Verbose adds much more, for a developer.
setting-log-domains-option-gpu = GPU
setting-log-domains-option-raw = Raw
setting-log-domains-option-pipe = Pipe
setting-log-domains-option-perf = Performance
setting-log-domains-option-memory = Memory
setting-log-domains-option-settings = Settings
setting-log-domains-option-control = Control
setting-log-domains-option-event = Events
setting-log-domains-option-verbose = Verbose

## Settings › Storage, its hand-built part (decision 76). $flag and $variable name what
## chose a folder; $kind is thumbnails, pipelines or libraries; $entries, $size and $unit
## as the cache's Clear has them; $limit the limit in GB.

settings-group-folders = Folders
settings-group-cache-contents = In the cache
settings-storage-user-data = User data folder
settings-storage-cache = Cache folder
settings-storage-chosen-flag = Chosen by { $flag }
settings-storage-chosen-variable = Chosen by { $variable }
settings-storage-chosen-user-data = Inside the user data folder
settings-storage-chosen-platform = The system's folder
settings-storage-none = None: the system gives no folder
settings-storage-show-tip = Open this folder
# Show's full name for screen readers: $show is its words, $folder the line's label.
settings-storage-show-name = { $show }: { $folder }
settings-storage-kind-thumbnails = Thumbnails
settings-storage-kind-pipelines = Pipelines
settings-storage-kind-libraries = Library caches
settings-storage-entries =
    { $kind ->
        [thumbnails]
            { $entries ->
                [one] { $entries } thumbnail
               *[other] { $entries } thumbnails
            }
        [libraries]
            { $entries ->
                [one] { $entries } library
               *[other] { $entries } libraries
            }
       *[pipelines]
            { $entries ->
                [one] { $entries } pipeline
               *[other] { $entries } pipelines
            }
    }
settings-storage-usage = { settings-storage-entries }, { job-cache-size }
settings-storage-total = Total
settings-storage-total-limit = { job-cache-size } of { $limit } GB
settings-storage-total-unlimited = { job-cache-size }, unlimited
settings-storage-counting = Counting…
settings-storage-clear = Clear
settings-storage-clear-thumbnails = Clear the thumbnails
settings-storage-clear-pipelines = Clear the pipelines
settings-storage-clear-tip = Removed from the disk, and made again when needed
settings-storage-no-cache = There is no disk cache: its folder could not be written as Apertrust started, so nothing is kept between runs.
settings-storage-refused-clearing = These are being cleared already.
settings-storage-refused-no-cache = There is no disk cache to clear.
settings-storage-refused = They could not be cleared.

## The log window, Help › Show Log (docs/gui.md, The log window); its filters are the
## Log row's options. $count, $kept and $shown are numbers of lines.

action-show-log = Show Log
# The window's title, and its name for screen readers.
log-name = Log
log-off-title = No log is being kept
log-off-text = Turn it on in Settings, and lines are kept from then on, in memory, for this session.
log-open-settings = Open Diagnostics settings…
# The bar of checkboxes that filter what is shown, and its first box.
log-filters = Show
log-filter-warnings = Warnings and errors
# The tooltip of a filter whose lines the log does not keep.
log-filter-not-kept = Not kept: turn it on in Settings › General › Diagnostics
# The lines, as screen readers name them.
log-lines = Lines
# The first line, once the oldest have gone.
log-dropped =
    { $count ->
        [one] The oldest line has gone: the log keeps the newest 20,000.
       *[other] The oldest { $count } lines have gone: the log keeps the newest 20,000.
    }
# The pill that goes back to the newest lines while the view is scrolled up.
log-newer =
    { $count ->
        [one] ↓ 1 newer line
       *[other] ↓ { $count } newer lines
    }
log-counts =
    { $kept ->
        [one] 1 line kept
       *[other] { $kept } lines kept
    } · { $shown } shown · this session only, in memory
log-names-files = Lines name your files and folders.
log-save = Save to file…
log-copy = Copy to clipboard
# The save dialog's title.
log-save-title = Save the Log

## About Apertrust

# The action, its item in the menus, and the dialog's name for screen readers.
action-about = About { app-name }
# $version is the build's version, as 0.1.0+197~g9201df0.
about-version = Version { $version }
# The version line's tooltip: a click copies the version alone, as --version prints it.
about-copy-version = Click to copy the version
# At the version line for a moment after a click copied it, and what screen readers hear then.
about-version-copied = Version copied
about-copyright = Copyright © 2026 { app-name } developers.
about-free-software = Free software under the GNU General Public License, version 3 or later.
about-credits = Credits
about-licence = Licence
# The button at the start of Credits and Licence that goes back to About.
about-back = Back

## Actions, as the keyboard map names them (docs/gui.md, Keyboard and actions)

# File › Open…: raws, their folder opened with the first in place of the photo open.
action-open = Open…
    .toolbar = Open
action-view-edit = Switch to Edit
# View › Collections: the first tab holding Collections.
action-show-collections = Collections
    .toolbar = Collections
# The Tools panel's tabs by position, whatever they hold.
action-panel-tab-1 = Tools panel tab 1
action-panel-tab-2 = Tools panel tab 2
action-panel-tab-3 = Tools panel tab 3
action-panel-tab-4 = Tools panel tab 4
action-panel-tab-5 = Tools panel tab 5
action-panel-tab-6 = Tools panel tab 6
action-panel-tab-7 = Tools panel tab 7
action-panel-tab-8 = Tools panel tab 8
action-panel-tab-9 = Tools panel tab 9
action-panel-tab-10 = Tools panel tab 10
action-panel-tab-11 = Tools panel tab 11
action-panel-tab-12 = Tools panel tab 12
# The Hidden tab, after the twelve, while it shows (decision 96).
action-panel-tab-hidden = Hidden tab
# The Tools panel's ⋮: the Hidden tab shown last and chosen, or taken away.
# Find Tool…, in the Tools panel's ⋮: the tab bar becomes a search field (decision 96).
action-find-tool = Find tool…
action-tools-panel-show-hidden = Show hidden tools
# A section's layout (decisions 83 and 96), each acting on the section whose header has the
# focus, or in ⋯ on its own: the words carry "Tool", so each reads on its own in the keymap.
# Pin Tool is Unpin Tool while the tool is pinned.
action-section-pin = Pin Tool
action-section-unpin = Unpin Tool
action-section-move-up = Move Tool Up
action-section-move-down = Move Tool Down
action-section-move-to-tab-1 = Move Tool to Tab 1
action-section-move-to-tab-2 = Move Tool to Tab 2
action-section-move-to-tab-3 = Move Tool to Tab 3
action-section-move-to-tab-4 = Move Tool to Tab 4
action-section-move-to-tab-5 = Move Tool to Tab 5
action-section-move-to-tab-6 = Move Tool to Tab 6
action-section-move-to-tab-7 = Move Tool to Tab 7
action-section-move-to-tab-8 = Move Tool to Tab 8
action-section-move-to-tab-9 = Move Tool to Tab 9
action-section-move-to-tab-10 = Move Tool to Tab 10
action-section-move-to-tab-11 = Move Tool to Tab 11
action-section-move-to-tab-12 = Move Tool to Tab 12
action-section-hide = Remove Tool
action-section-show = Show Tool in Tab
# A tool's options menu's editing items: Reset does what its header's reset button does.
action-section-reset = Reset
action-section-copy-settings = Copy Settings
action-section-paste-settings = Paste Settings
action-scale-up = Larger interface
action-scale-down = Smaller interface
action-scale-reset = Normal interface size
# F6 moves the focus between the Toolbar, the viewer bar, the Tools panel and the Browser.
action-next-region = Next pane
action-previous-region = Previous pane
action-full-screen = Full screen
    .toolbar = Full screen
# File › Close Window on the Mac: the window with the focus, Settings' or the main window,
# which quits.
action-close-window = Close window
# File › Close: the folder or photo open, its edit saved; then where to start.
action-close = Close
# File › Open Recent's items by place, newest first, as the keyboard map names them;
# the menu shows each folder's name. Clear Menu empties the list.
action-open-recent-1 = Open recent 1
action-open-recent-2 = Open recent 2
action-open-recent-3 = Open recent 3
action-open-recent-4 = Open recent 4
action-open-recent-5 = Open recent 5
action-open-recent-6 = Open recent 6
action-open-recent-7 = Open recent 7
action-open-recent-8 = Open recent 8
action-open-recent-9 = Open recent 9
action-open-recent-10 = Open recent 10
action-clear-recent = Clear menu
action-quit = Quit
action-restart = Restart
action-peek = See the photo without the section under the pointer
action-next-photo = Next photo
    .toolbar = Next
action-previous-photo = Previous photo
    .toolbar = Previous
action-zoom-in = Zoom in
action-zoom-out = Zoom out
action-zoom-fit = Zoom to fit
    .toolbar = Fit
action-zoom-100 = Zoom to 100%
    .toolbar = 100%
action-zoom-small = Zoom to small
action-zoom-fill = Zoom to fill
action-zoom-50 = Zoom to 50%
action-zoom-200 = Zoom to 200%
action-zoom-400 = Zoom to 400%
action-zoom-800 = Zoom to 800%
action-zoom-1600 = Zoom to 1600%
action-zoom-in-all = Zoom in on every photo
action-zoom-out-all = Zoom out on every photo
action-zoom-fit-all = Fit every photo
action-zoom-100-all = Every photo to 100%
# Alt with the arrows: a zoomed photo's view moved a fifth of the part shown, or with
# Shift a whole view.
action-pan-left = Move the view left
action-pan-right = Move the view right
action-pan-up = Move the view up
action-pan-down = Move the view down
action-pan-page-left = Move the view a screen left
action-pan-page-right = Move the view a screen right
action-pan-page-up = Move the view a screen up
action-pan-page-down = Move the view a screen down
# The browser bar's menus, and View › Sort By and Thumbnail Size: the sort's key and
# direction, and the thumbnails' size.
action-sort-capture-time = Capture time
action-sort-file-name = File name
action-sort-edited = Edited
action-sort-rating = Rating
action-sort-ascending = Ascending
action-sort-descending = Descending
action-thumbnails-small = Small
action-thumbnails-medium = Medium
action-thumbnails-large = Large
action-thumbnails-extra-large = Extra large
# Where the Browser goes: the browser bar's options menu and View › Browser Position.
action-browser-position-left = Left
action-browser-position-right = Right
action-browser-position-bottom = Bottom
# Where the Tools panel goes: its ⋮'s Position and View › Tools Panel Position.
action-tools-panel-position-left = Left
action-tools-panel-position-right = Right
# View › Show Labels and the browser bar's options menu: the captions under the thumbnails.
action-show-labels = Show labels
action-background-black = Black
action-background-very-dark = Very dark
action-background-dark = Dark
action-background-medium = Medium
action-background-light = Light
action-background-white = White
# View › Guides (decision 92): Guides on or off, darktable's guide types, its flips.
action-guides = Guides
    .toolbar = Guides
action-guides-grid = Grid
action-guides-rules-of-thirds = Rules of thirds
action-guides-metering = Metering
action-guides-perspective = Perspective
action-guides-diagonal-method = Diagonal method
action-guides-harmonious-triangles = Harmonious triangles
action-guides-golden-sections = Golden sections
action-guides-golden-spiral = Golden spiral
action-guides-golden-spiral-sections = Golden spiral sections
action-guides-golden-mean = Golden mean (all guides)
action-guides-flip-none = None
action-guides-flip-horizontally = Horizontally
action-guides-flip-vertically = Vertically
action-guides-flip-both = Both
# Flip's items' tooltip while the guide type is one darktable never flips.
guides-flip-unavailable = Only harmonious triangles and the golden guides flip
# The Toolbar's Guides' tooltip, a line after its words and keys: what its right-click
# opens (decision 92).
toolbar-guides-menu = Right-click for the guide type
# The space bar, as a shortcut names it: "Space", "Ctrl+Space".
key-space = Space

## The menus' titles, in title case as menus are (docs/gui.md, The menu bar); the app
## menu's is app-name

# The & marks the letter that opens the menu with Alt on Windows and Linux, its access key
# (decision 39): each menu's its own in each language, typed without AltGr. The Mac shows
# none. File's word is also a term, for sentences that name the menu; Fluent keeps terms
# and messages under one name, so the term's is another.
-file-menu = File
menu-file = &{ -file-menu }
menu-edit = &Edit
menu-select = &Select
menu-photo = &Photo
menu-view = &View
menu-window = &Window
menu-help = &Help
# Submenus of View.
menu-tools-panel-tab = Tools Panel Tab
menu-workspace = Workspace
# View › Panels: the panels' own items, the Toolbar's among them (decision 92).
menu-panels = Panels
# A tool's ⋯, its layout items (decisions 83 and 96): its tabs by name under each.
menu-section-move-to-tab = Move Tool to Tab
menu-section-show-in-tab = Show Tool in Tab
# Remove Tool's tooltip; $tool is the tool's name. Export's is drawn disabled, saying why.
section-hide-tip = Takes { $tool } out of this workspace's tabs. Its edit stays; Show Hidden Tools shows it.
section-hide-export = Export stays in every workspace
# Move Tool Up's tooltip where the tool is first in its list, and Move Tool Down's where
# it is last; $tool is the tool's name, $list "pinned" for the tab's pinned list, else
# "other".
section-move-up-first =
    { $list ->
        [pinned] { $tool } is already at the top of the pinned tools
       *[other] { $tool } is already at the top
    }
section-move-down-last =
    { $list ->
        [pinned] { $tool } is already at the bottom of the pinned tools
       *[other] { $tool } is already at the bottom
    }
menu-interface-size = Interface Size
menu-sort-by = Sort By
menu-thumbnail-size = Thumbnail Size
menu-background = Background
menu-guides = Guides
menu-guides-flip = Flip
menu-browser-position = Browser Position
menu-tools-panel-position = Tools Panel Position
# A submenu of File, and its item for a recent whose name another shares: with the
# name of the folder it is in.
menu-open-recent = Open Recent
menu-recent-in = { $name } — { $parent }
# Edit › Undo and Redo, naming what they would do as History names the step: "Undo
# Exposure", "Undo Go to Step". With nothing to name they read action-undo and action-redo.
menu-undo-step = Undo { $step }
menu-redo-step = Redo { $step }
# File › Export… and Export Again while more than one photo is selected: how many they
# export, $count, every selected photo.
menu-export-photos =
    { $count ->
        [one] Export { $count } Photo…
       *[other] Export { $count } Photos…
    }
menu-export-photos-again =
    { $count ->
        [one] Export { $count } Photo Again
       *[other] Export { $count } Photos Again
    }
# The Mac's own items in the app menu and the Window menu, which macOS performs.
menu-services = Services
menu-hide-app = Hide { app-name }
menu-hide-others = Hide Others
menu-show-all = Show All
menu-minimise = Minimise
menu-zoom-window = Zoom
menu-bring-all-to-front = Bring All to Front

## Context menus, by what screen readers call them (decision 91): the photo's, a
## thumbnail's, a History step's and a job's row's in the jobs window

context-menu-photo = Photo
context-menu-thumbnail = Thumbnail
# $number is the step's, as History numbers it.
context-menu-step = Step { $number }
context-menu-job = Job

## The edit's history: a step's name, as History lists it and Undo and Redo name it

# A step that put the whole edit back to nothing: Reset Edit.
history-reset = Reset
# The steps of one change that touched several tools, a step each, as Undo and Redo name
# them: $tool is the first's title, $count how many more there are.
history-tools = { $tool } and { $count } more
# Going to a step in History, which Undo takes back and Redo puts back.
history-go-to-step = Go to Step
# Compress History, Truncate History and Discard History, which Undo takes back and Redo
# puts back; the steps Discard History starts an unreadable history afresh with.
history-compress = Compress History
history-truncate = Truncate History
history-discard = Discard History
# The History section's options (⋯), as items, each with its tip: Compress History,
# Truncate History, Create a Style…, which comes with styles, and Discard History…'s tip.
history-options-compress = Compress History
history-options-truncate = Truncate History
history-compress-tip = Keeps each tool’s last step up to the chosen one and removes the rest. The photo stays as it is, but at Original, where the whole history goes and the photo starts again from its defaults.
history-truncate-tip = Removes the steps after the chosen one and keeps the rest as they are. The photo stays as it is, but at Original, where the whole history goes and the photo starts again from its defaults.
history-options-style = Create a Style…
history-options-style-later = Styles come in a later version.
history-discard-tip = Clears the history and puts the photo back as it first opened.
# The same on an edit part of whose history can't be read, which it starts afresh.
history-discard-tip-unreadable = Starts the history afresh from the photo as it looks now. What can’t be read is lost.
# Why Discard History… is disabled where it would ask and the system's question can't be
# shown yet, Windows' until its alert is built: as export-conflict-overwrite-no-alert.
history-discard-no-alert = Discard History asks first, and that question can’t be shown here yet. Turn off Ask before discarding a photo’s history in Settings to discard without asking.
# The same on an edit part of whose history can't be read, which always asks first.
history-discard-no-alert-unreadable = Discarding this history asks first, since nothing can bring back what can’t be read, and that question can’t be shown here yet.
# A History step's right-click menu, as items, each with its tip: Go to Step, Show Tool,
# which opens the step's tool, Compress up to Here and Truncate after Here, which go to the
# step first.
history-step-go-to = Go to Step
history-step-go-to-tip = Shows the photo as it was at this step. The steps after it stay until your next edit replaces them.
history-step-show-tool = Show Tool
history-step-show-tool-tip = Opens this step’s tool in the Tools panel, as Shift-click on the step does.
history-step-compress = Compress up to Here
history-step-compress-tip = Goes to this step, then keeps each tool’s last step up to it and removes the rest.
history-step-truncate = Truncate after Here
history-step-truncate-tip = Goes to this step, then removes the steps after it and keeps the rest as they are.

## The History section in the Tools panel's History tab (docs/history.md)

section-history = History
    .search = history
# Its header's reset button, which puts the whole edit back as one step, as Reset Edit.
history-reset-edit = Reset edit
# A step's name for screen readers, with its number: VoiceOver reads a list item's name
# alone.
history-step-name = Step { $number }: { $name }
# The line at the bottom, 0: the photo with no step applied.
history-original = Original
history-original-tip = The photo with none of these steps
# Each line's tag: who made the step.
history-source-you = You
history-source-auto = Auto
history-source-style = Style
history-source-assistant = Assistant
history-source-default = Default
# A step's tooltip, a row a control, its label and its values before and after drawn
# apart; for screen readers, a control changed: its label, its value before and after.
history-said-change = { $control }, { $before } to { $after }

# The same, the control being the tool's own, its label left out.
history-said-change-alone = { $before } to { $after }
# What changed, after a step's tool in its line: several controls, how many.
history-line-controls =
    { $count ->
        [one] { $count } control
       *[other] { $count } controls
    }

# A step's name for screen readers with what it changed.
history-step-name-change = Step { $number }: { $name }, { $change }
# A change of several tools, a step each, shown as one line that opens: its name where
# it has no note, how many tools it touched.
history-group-tools =
    { $count ->
        [one] { $count } tool
       *[other] { $count } tools
    }
# That line's name for screen readers: its steps' first and last numbers, its name, then
# its tools' titles.
history-group-name = Steps { $first } to { $last }: { $name }, { $tools }
# The tools every photo runs, shown as one line over Original that opens, and how many
# they are.
history-defaults = Defaults
history-defaults-tools =
    { $count ->
        [one] { $count } tool every photo runs
       *[other] { $count } tools every photo runs
    }
# A line's name for screen readers with what it says after its name: the Defaults line's.
history-line-name = { $name }, { $change }
# Over each day's steps: today's, yesterday's with its date, then each earlier day's date
# alone; $date as "12 November", with its year where it is another year's.
history-day-today = Today
history-day-yesterday = Yesterday, { $date }
# Over steps whose time is unknown, as a version 1 edit's whose file's time was unknown,
# under a day's steps.
history-day-unknown = No date
# Under the steps after the end, which the next edit drops.
history-undone =
    { $count ->
        [one] ↑ { $count } undone · your next edit replaces it
       *[other] ↑ { $count } undone · your next edit replaces them
    }
# The undone mark as screen readers hear it, with no arrow or dot.
history-undone-said =
    { $count ->
        [one] { $count } undone: your next edit replaces it
       *[other] { $count } undone: your next edit replaces them
    }
# A control whose value stayed, its marker changed.
history-kept = { $control } kept
history-released = { $control } released
history-auto = { $control } set by Auto
# A checkbox's value, and a curve's: how many points it has.
history-on = On
history-off = Off
history-points =
    { $count ->
        [one] { $count } point
       *[other] { $count } points
    }
# The eye at the end of a step's line, held to see the photo without its tool for that
# moment, never saved: its name for screen readers, then its tooltip; $tool is the
# tool's title.
history-peek = Hold to see the photo without { $tool }
history-peek-tip = Hold to see the photo without { $tool }; nothing is saved

## The curve; $control is its full name

# A point's name for screen readers, with where it is: VoiceOver reads a list item's name
# alone. $index counts from 1; $input and $output are percentages.
curve-point = Point { $index }: input { $input }%, output { $output }%
# Shown over the curve while it is in use.
curve-point-value = Input { $input }%, output { $output }%
curve-black-point = { $control }: black point
curve-white-point = { $control }: white point
# What the keys do, for screen readers, with the keys spelt as printed on them.
curve-keys = Page Up and Page Down choose a point, the arrow keys move it, Insert or + adds one, Delete removes it and Shift+Delete removes them all.
# The same on a Mac, which has no Insert key and calls Backspace Delete.
curve-keys-mac = Page Up and Page Down (Fn+↑ and Fn+↓) choose a point, the arrow keys move it, + adds one, Delete removes it and Shift+Delete removes them all.

## The tone equalizer's graph of its bands; $tool is the tool's title

# The graph's full name, for screen readers.
equalizer-name = { $tool }, curve
# Its tooltip: darktable's.
equalizer-tip = Double-click to reset the curve.
# What the keys do, for screen readers, with the keys spelt as printed on them.
equalizer-keys = Page Up and Page Down choose a band, the arrow keys move it, Delete resets it and Shift+Delete resets them all.
# The same on a Mac, which has no Page Up and Page Down keys of their own.
equalizer-keys-mac = Page Up and Page Down (Fn+↑ and Fn+↓) choose a band, the arrow keys move it, Delete resets it and Shift+Delete resets them all.
# Where the mask reaches past the bands, which a warning sign shows on the graph, for
# screen readers.
equalizer-overflows-shadows = The mask's shadows go past the −8 EV band.
equalizer-overflows-highlights = The mask's highlights go past the +0 EV band.
equalizer-overflows-both = The mask's shadows go past the −8 EV band, and its highlights past the +0 EV band.

## The scopes; $shadows and $highlights are shares of the photo's pixels, in percent

# The section that holds them, pinned at the top of the Tools panel's editing tabs.
section-scopes = Scopes
    .search = scopes
scope-histogram = Histogram
scope-waveform = Waveform
scope-parade = RGB parade
scope-vectorscope = Vectorscope
# The buttons that choose among them, for screen readers.
scope-graphs = Graph
# What each graph shows, in its tooltip.
scope-histogram-tip = How many pixels have each brightness: black on the left, white on the right.
scope-waveform-tip = Brightness across the photo, left to right as in the photo: dark at the bottom, bright at the top.
scope-parade-tip = The red, green and blue waveforms side by side. Where they line up, the photo is neutral.
scope-vectorscope-tip = Colours by hue, the angle, and by saturation, the distance from the centre.
scope-log-scale = Logarithmic scale
scope-red = Red channel
scope-green = Green channel
scope-blue = Blue channel
scope-luminance = Luminance
scope-show-shadow-clipping = Show shadow clipping
scope-show-highlight-clipping = Show highlight clipping
# What clips, for screen readers.
scope-no-clipping = No clipping
scope-clipped-shadows = Shadows clipped: { $shadows }%
scope-clipped-highlights = Highlights clipped: { $highlights }%
scope-clipped-both = Shadows clipped: { $shadows }%, highlights clipped: { $highlights }%
# Before anything has been measured for the graph.
scope-empty = Nothing measured yet

## Colour pickers (docs/pickers.md, The picker button). $tip is the picker's own words,
## $command the pointer's modifier as shortcuts spell it: "Cmd" on the Mac, "Ctrl" elsewhere

# A picker button's tooltip where its tool gives none, darktable's "pick color from image".
picker-tip = Pick colour from the photo
# The tooltip of a picker that takes a point or an area: its words, then darktable's
# "ctrl+click or right-click to select an area".
picker-tip-area =
    { $tip }
    { $command }+click or right-click to select an area
# The same on a slider's quad, which takes every mouse button alike: darktable's "ctrl+click
# to select an area".
picker-tip-area-quad =
    { $tip }
    { $command }+click to select an area

## The active picker on the photo (docs/pickers.md, On the photo, Accessibility). $tool is
## the tool's title, $picker the picker's name: its control's label on a slider's quad,
## else its own; $command is the pointer's modifier as shortcuts spell it, $what the
## picker's own sentence

# The viewer bar's start while a picker is on.
picker-bar-name = { $tool } · { $picker }
# The label naming the tool on the picker's area or point, where the tool has several
# pickers: "Exposure · Reference".
picker-area-label = { $tool } · { $picker }
# How to use a picker measuring an area.
picker-bar-area = Drag an area on the photo. { $command }-click switches to a point. Right-click resets the area. Esc ends.
# How to use a picker measuring a point, after its own sentence.
picker-bar-point = { $what } { $command }-click switches to an area. Esc ends.
# The sentence of a picker with none of its own: an area picker switched to a point.
picker-bar-point-what = Click or drag the point on the photo.
# The picker on the photo, as screen readers hear it, and its value in percent of the photo.
picker-photo-area = Picker area, { $picker }
picker-photo-point = Picker point, { $picker }
picker-photo-at-point = point at { $x }%, { $y }%
picker-photo-at-area = area from { $left }%, { $top }% to { $right }%, { $bottom }%
picker-photo-up = Move up
picker-photo-down = Move down
# What is read out once a pick has rested: each control it changed with its value, or the
# colour it measured, each channel 0 to 255.
picker-said-control = { $control } { $value }
picker-said-rgb = mean RGB { $red }, { $green }, { $blue }

# Said politely as a picker turns on, from its button or its cursor, and as it turns off
# where the person did not ask, with why where the app knows (docs/cursors.md, Keyboard and
# screen readers). $picker is its name, "White balance picker".
picker-turned-on = { $picker } on
picker-turned-off = { $picker } off
picker-turned-off-why = { $picker } off: { $why }
# Said as another picker replaces the one on, $before the one turning off.
picker-turned-replaced = { $before } off: another picker turned on. { $picker } on
# Why a picker turned off: another photo opened; the photo closed; a control of its tool
# changed, $control its label; a tool changed, $tool its name; its tool turned off; its
# tool's section closed; Undo, Redo or a move through the history; the edit reset; the
# reference it picks toward cleared.
picker-off-photo = another photo
picker-off-closed = photo closed
picker-off-control = { $control } changed
picker-off-tool = { $tool } changed
picker-off-tool-off = { $tool } turned off
picker-off-section = { $tool } closed
picker-off-history = history changed
picker-off-reset = edit reset
picker-off-reference = reference cleared

## The cursors' well (docs/cursors.md, The well; decision 104). $name is a cursor's name,
## $key its key as shortcuts spell it, $how what a press with it does, $group a group's name

# The well, as screen readers hear it.
cursor-well = Cursors
# A face, as screen readers hear it: its cursor's name, then its key where it has one.
cursor-name-key = { $name }, { $key }
# A face's tooltip: its cursor's name, key and what it does; without a key; without words
# of what it does.
cursor-tip = { $name } · { $key } — { $how }
cursor-tip-how = { $name } — { $how }
cursor-tip-key = { $name } · { $key }
# A group's chevron and the list it opens: $has is what the list holds, the group's
# cursors, its settings alone (a group of one), or both.
cursor-list =
    { $has ->
        [cursors] { $group } cursors
        [settings] { $group } settings
       *[both] { $group } cursors and settings
    }

# The app's own cursors and the groups, their names and what a press with each does; a
# group's name says what its chevron's list holds.
cursor-select = Select
cursor-select-how = Click a mark on the photo to select it, drag to move it
cursor-pan = Pan
cursor-pan-how = Drag to pan a zoomed photo, over marks too; or hold A with any cursor
cursor-group-select = Select
cursor-group-pan = Pan
cursor-group-pick = Pick
# The well's tooltip while no photo is shown.
cursor-well-no-photo = Open a photo to choose a cursor

## A picker's patches (docs/pickers.md, Declaring pickers): darktable's input and target
## patches under an area mapping. $value is the input's L, in the language's number format
## with one decimal.
picker-patch-input = Input
picker-patch-input-tip = The input colour that should be mapped to the target.
picker-patch-target = Target
picker-patch-lch-tip = These LCh coordinates are computed from CIE Lab 1976 coordinates.
picker-patch-lightness = L: { $value }%
picker-patch-none = L: N/A
picker-patch-hue = h: { $value } °
picker-patch-hue-none = h: N/A
picker-patch-chroma = c: { $value }
picker-patch-chroma-none = c: N/A

## A reference row (docs/pickers.md, Declaring pickers; decision 88): an area's L kept across
## photos: its L, $lightness in the language's numbers, then the photo it was measured on,
## $photo its name, drawn after the L as a part of its own, dimmed once the photo is gone.
reference-value = L { $lightness } %
reference-photo = · { $photo }
reference-this-photo = · this photo
reference-none = none
reference-measuring = measuring…
reference-gone = The photo it was measured on is no longer there
reference-more = Reference options
reference-clear = Clear Reference
reference-set = Set Reference from an Area

## A last pick row (docs/pickers.md, Declaring pickers; decision 88): what the last pick did,
## $before and $after the area's L and $change the change with its unit, each ■ where the
## row draws a patch of the area's colour, then its two
## warnings, $control the control's label, $value the end it is held at with its unit,
## $reached and $target the L it reached and the L it was to reach.
last-pick-label = Last pick
last-pick-value = ■ L { $before } % → ■ L { $after } % · { $change }
last-pick-promise = The area picked, before and after, and the exposure it took
pick-warning-clipped = The area has clipped pixels
pick-warning-clipped-tip = Its lightness reads low: pick an area with no clipped highlights
pick-warning-short = { $control } at { $value }: the area is L { $reached } %, not { $target } %
pick-warning-short-tip = { $control } reached its end of { $value }

## The Export tool (docs/gui.md, The Export tool): darktable's export module, its
## labels in sentence case. $count is the photos, a single one the exact number 1. $file
## is a photo's name or a file's, $source the raw, $folder a folder, $width and $height
## pixels, $problem what stops the export.

export-button = Export { photo-count }
export-reset = Reset export settings
export-presets = Export presets
export-presets-later = Presets come in a later version
export-preset-default = Default settings
export-metadata-note = This version writes no EXIF, location or copyright into exported files.
export-path-tip-unexact = { $source } → { $pattern }, filled in as it is exported

## The Summary, the warnings, the button and the line under it

export-summary-title = Summary
export-summary-label-preset = Preset
export-summary-label-file-name = File name
export-summary-label-size = Size
export-summary-label-scale = Scale
export-summary-label-profile = Profile
export-summary-label-format = Format
export-summary-label-destination = Destination
export-summary-size = { $width } × { $height } px
export-summary-size-print = { $width } × { $height } { $unit } ({ $pixels_width } × { $pixels_height } px)
export-summary-scale = { $percent }%
export-summary-jpeg = JPEG, quality { $quality }
export-summary-png = PNG, { $bits } bit

export-view-only-left-out =
    { $count ->
        [one] { $count } view-only photo left out
       *[other] { $count } view-only photos left out
    }
export-warn-replaces =
    { $count ->
        [one] Replaces { $count } file already there
       *[other] Replaces { $count } files already there
    }
export-warn-replaces-at-least =
    { $count ->
        [one] Replaces at least { $count } file already there
       *[other] Replaces at least { $count } files already there
    }
export-show-jobs = Show jobs
export-failed = Export failed: { $done } of { $total }

# The tooltip of every row and group drawn disabled because it is not built yet.
export-later = Coming in a later version

## Storage options

export-storage-options = Storage options
export-target-storage = Target storage
export-storage-latex = LaTeX book template
export-storage-disk = File on disk
export-storage-email = Send as email
export-storage-gallery = Website gallery
export-path = Path
export-choose-folder = Choose folder
# The folder button's dialog: its title, and its accept button where the platform lets the
# app name it (darktable's "select directory" and "select as output destination").
export-choose-folder-title = Select Folder
export-choose-folder-accept = Select as Output Destination
# What the path field's list is called, for screen readers.
path-variables = Variables
export-on-conflict = On conflict
export-conflict-unique = Create unique filename
export-conflict-overwrite = Overwrite
export-conflict-changed = Overwrite if changed
export-conflict-skip = Skip
export-conflict-overwrite-no-alert = Overwrite asks before it replaces files, and that question can’t be shown here yet. Choose another way.

## What the session says under the path and On conflict

# $source is the raw, $file where its file goes.
export-name = { $source } → { $file }
export-names-more = and { $count } more, the same way
export-no-name = The path gives no file name for this photo.
export-sequence-added = With { $count } photos and no variable in the path, _$(SEQUENCE) is added so that none overwrites another.
# $names are variables darktable has and Apertrust has no value for.
export-not-here = { $names }: darktable's, with no value here; it expands to nothing.

## What stops the export (D8)

export-problem-empty = Enter where to put the exported photos.
export-problem-nameless = A variable's name is missing after “$(”. Type $( to see the variables.
export-problem-unknown = Unknown variable $({ $name }). Type $( to see the variables.
export-problem-unknown-near = Unknown variable $({ $name }). Did you mean $({ $suggestion })?
export-fix = Use $({ $name })
export-problem-unclosed = Missing “{ $expected }” after “{ $variable }”.
export-problem-no-operation = “{ $found }” after { $variable } is no operation.
export-problem-too-deep = Variables are nested more than { $depth } deep.
export-problem-not-connected = Can’t write to “{ $folder }”: the disk isn’t connected. Connect it, or choose another folder.
export-problem-not-a-folder = Can’t write to “{ $folder }”: a file is in the way.
export-problem-read-only = Can’t write to “{ $folder }”: it is read-only.
export-problem-unreadable = Can’t write to “{ $folder }”: { $why }.
export-problem-too-large = { $format } is at most { $largest } px a side.
export-problem-no-pixels = The size leaves less than a pixel: { $width } × { $height } px.
export-problem-failed = The photo can’t be exported at this size: { $why }
# The tooltip of Export while it cannot run; $problem is one of the messages above.
export-why-path = Fix the path first: { $problem }
export-why-size = Fix the size first: { $problem }
export-why-view-only-newer = A newer Apertrust wrote these edits.
export-why-view-only-unreadable = The history in { $file } can’t be read.

## Format options

export-format-options = Format options
export-file-format = File format
export-format-copy = Copy
export-jpeg-quality = Quality
export-jpeg-subsample = Chroma subsampling
export-jpeg-subsample-tip = Chroma subsampling for the JPEG encoder. Auto: from the quality. 4:4:4: none. 4:4:0: colour halved vertically. 4:2:2: halved horizontally. 4:2:0: halved both ways.
export-subsample-auto = Auto
export-png-bit-depth = Bit depth
export-png-compression = Compression
export-bits-8 = 8 bit
export-bits-16 = 16 bit

## Global options

export-global-options = Global options
export-set-size = Set size
export-mode-pixels = In pixels (for file)
export-mode-cm = In cm (for print)
export-mode-inch = In inch (for print)
export-mode-scale = By scale (for file)
export-size-pixels = Size in pixels
export-size-cm = Size in centimetres
export-size-inch = Size in inches
export-unit-cm = cm
export-unit-inch = in
export-width = Width
export-height = Height
export-dpi = Resolution
export-dpi-unit = dpi
export-equal-to = Which is equal to { $width } × { $height } px
export-scale = Scale
export-scale-tip = An integer, a decimal number or a simple fraction, such as 1/2. 0 or empty is 1.
# The active photo's size under the rows; $file is its name.
export-size = { $file } will be { $width } × { $height } px.
export-size-capped = { $file } will be { $width } × { $height } px (not upscaled: Allow upscaling is off).
export-size-full = { $file } will be { $width } × { $height } px (full size).
export-size-no-limit = { $file } will be { $width } × { $height } px (full size: no limit set).
export-upscale = Allow upscaling
export-high-quality = High quality resampling
export-high-quality-tip = Do high quality resampling during export: every operation at full size, then scaled down.
export-masks = Store masks
export-masks-later = Store masks as layers in exported images. Only TIFF, OpenEXR and XCF can, and layers come with a later version.
export-option-no = No
export-option-yes = Yes
export-profile = Profile
export-profile-image = Image settings
export-profile-tip = Image settings: the photo's own, in Raw › Output colour profile, as the view shows it. Another profile applies to this export alone.
export-intent = Intent
export-intent-perceptual = Perceptual
export-intent-relative = Relative colorimetric
export-intent-saturation = Saturation
export-intent-absolute = Absolute colorimetric
export-intent-tip = With matrix profiles and no ICC files the intent changes nothing, so it waits for ICC files.
export-style = Style
export-style-none = None
export-style-later = Styles come in a later version.
export-metadata = Metadata
export-multi-preset = Multi-preset export

## Alerts (docs/gui.md, Dialogs and alerts): the system's own dialogs ask these. $stoppable
## is how many of the running jobs a quit stops, $waiting how many it waits for.

alert-cancel = Cancel
alert-overwrite-title = Export in overwrite mode?
alert-overwrite-message = Exporting replaces the files already there that have the same names. The old files can’t be brought back.
alert-overwrite-button = Overwrite
alert-overwrite-suppress = Don’t ask again
alert-quit-title = Quit while a job is running?
alert-quit-message =
    { $stoppable ->
        [0] Apertrust quits once the running jobs have finished.
       *[other]
            { $waiting ->
                [0] The running jobs stop after the photo being written, then Apertrust quits. The files already written stay.
               *[other] The jobs that can be stopped stop after the photo being written, and Apertrust quits once the others have finished. The files already written stay.
            }
    }
alert-quit-button = Quit
# The Toolbar's save warning pressed: why the edit isn't saved, and Try Again, which
# writes it again. $name is the photo's file name, $error why, in the system's words.
alert-save-title = The edit isn’t saved
alert-save-message = The edit of { $name } couldn’t be written beside it: { $error }
alert-save-button = Try Again
# Quit after the edit's last write failed, Cancel the default; $error is why.
alert-quit-save-title = Quit without your last changes?
alert-quit-save-message = Your last changes couldn’t be saved: { $error }
alert-quit-anyway = Quit Anyway
# What was dropped onto the window opens nothing: $name is the first thing dropped, the
# message says why, and the button is the one way out. An edit (IMG.ARW.apxmp) opens its
# photo, $photo (IMG.ARW), which no-photo says is not beside it.
alert-refused-title = Can’t open { $name }
alert-refused-unknown = Only folders and raw photos open here.
alert-refused-no-photo = Its photo, { $photo }, isn’t in the same folder.
alert-refused-button = OK
# A second start of Apertrust on the same user data folder was given flags for one run
# ($flags, as typed: --safe-mode, -d gpu, --conf KEY=VALUE, --cache-dir DIR), which the app
# already running cannot take; this window, brought forward, says so.
alert-running-title = Apertrust is already running with this folder
alert-running-message = To start it with { $flags }, quit Apertrust first, then start it again.
alert-running-button = OK
# Discard History…, Cancel the default: darktable's "delete image's history?". Undo takes it
# back while the photo is open.
alert-discard-title = Discard this photo’s history?
alert-discard-message = The photo goes back to how it first opened, and its history is cleared. Undo brings it back until the photo closes.
# The same where part of the edit's history can't be read, so it opened for viewing only:
# discarding starts the history afresh from the photo as it looks, nothing takes it back, and
# what couldn't be read is lost. $file is the edit's file beside the photo.
alert-discard-unreadable-message = Some of this edit’s history can’t be read. Discarding starts it afresh from the photo as it looks now, and what can’t be read is lost for good. To keep it, copy { $file } first.
alert-discard-button = Discard History
