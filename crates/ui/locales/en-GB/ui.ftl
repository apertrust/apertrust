# Apertrust's interface in British English, the source language (docs/gui.md, Translation).
# Every message id says where the text appears. Ukrainian (uk) and Polish (pl) translate
# every message here, with the same variables.

app-name = Apertrust

## The top panel's common actions

action-import = Import…
action-export = Export…
# File › Export Again: the export with the settings as last kept, with no dialog.
action-export-again = Export Again
action-reset = Reset
action-undo = Undo
action-redo = Redo
action-copy-edit = Copy edit
action-apply-edit = Apply edit
action-copy-apply-parts = Copy or apply parts
action-settings = Settings
status-saved = Saved

## The side panel's tabs

# Edit's tabs: darktable's four module groups, base, color, correct and effect, between
# Apertrust's own Library and History (decision 38).
tab-library = Library
tab-base = Base
tab-color = Colour
tab-correct = Correct
tab-effect = Effect
tab-history = History
tab-metadata = Metadata
# The tooltip of a tab drawn disabled because nothing is behind it yet; $tab is its name.
panel-tab-later = { $tab }: coming in a later version
panel-tabs = Side panel tabs
panel-more-tabs = More tabs
panel-tab-options = Tab options
panel-show-tab-names = Show tab names
panel-customise-tabs = Customise tabs…
panel-width = Side panel width

## A section's header buttons; $section is the section's title

section-reset = Reset { $section }
section-presets = { $section } presets
section-options = { $section } options
section-toggle = Open or close { $section }. Hold H over it to see the photo without it.
# The name of a section's tab list, for screen readers.
section-tabs = { $section } tabs
# The tooltip of a tab drawn disabled because nothing is behind it yet.
section-tab-later = Coming in a later version
# The tooltip of a choice's option drawn disabled, as one darktable lists that is not
# ported yet.
choice-option-later = Coming in a later version
# The tooltip and description of a choice's option an edit from another photo holds,
# which this photo does not list (decision 30).
choice-option-unavailable = Not available for this photo

## A collapsible group inside a module, darktable's collapsible section

# What screen readers hear after its title while controls in it are off their defaults;
# $count is how many.
collapsible-group-set = { $count } set
# The tooltip of a closed group while controls in it are set; $group is its title and
# $count how many.
collapsible-group-tip = { $group }: { $count } set

## A preset row in a module, whose options each write several of its controls, as White
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

## A value with its unit, as a slider shows it and screen readers hear it; $value is
## already in the language's number format, signed where the slider's range spans zero

unit-ev-value = { $value } EV
unit-percent-value = { $value }%
unit-degrees-value = { $value }°
unit-kelvin-value = { $value } K
unit-pixels-value = { $value } px
unit-iso-value = { $value } ISO

## The top panel

top-panel = Top panel
# ☰ at the top panel's start on Windows and Linux, which opens the menus.
top-panel-menu = Menu
# ⋯, holding the common actions a narrow top panel has no room for.
top-panel-more = More actions
status-unsaved = Not saved yet
# Where Saved sits, while the photo open is open for viewing only: a newer Apertrust
# wrote its edit, which this one never saves.
status-view-only = View only
# The window's caption buttons at the end of the top panel, where the app draws them.
window-minimise = Minimise
window-maximise = Maximise
window-restore = Restore
window-close = Close

## The jobs: the spinner at the top panel's end and the window it opens. An export's row keeps
## the activity's words below. $running, $waiting and $failed count jobs, $value is the
## spinner's value, $message a job's own message, $done and $total its numbers and $target
## where an export goes. The spinner's value avoids plural forms: "2 running".

jobs-name = Jobs
jobs-tip = Jobs: { $value }
jobs-value-running = { $running } running
jobs-value-waiting = { $waiting } waiting
jobs-value-running-waiting = { $running } running, { $waiting } waiting
jobs-value-finished = Finished
jobs-value-cancelled = Cancelled
jobs-value-failed = { $failed } failed
jobs-empty = No jobs.
jobs-queued = Waiting for other jobs
jobs-finished = Finished
jobs-failed = Failed
jobs-cancelled = Cancelled
jobs-cancel = Cancel
jobs-cancel-tip = Stop this job
# What the live region says as a job starts or ends.
jobs-live-started = Started: { $message }
jobs-live-finished = Finished: { $message }
jobs-live-failed = Failed: { $message }
jobs-live-cancelled = Cancelled: { $message }

## The jobs' own messages, as darktable words its jobs

job-export = Exporting { $done } / { $total } to { $target }
job-import = Importing { $done } / { $total }
job-delete = Deleting { $done } / { $total }
job-thumbnails = Creating thumbnails
# A folder being read as it opens; $folder is its path.
job-folder = Reading { $folder }
# The storage an export goes to, in the export's message.
job-target-disk = file on disk

## Why a job failed, in words, without a full stop: a code the engine sends. They end a
## line such as "IMG_2047.ARW: could not write IMG_2047.jpg: the disk is full."

job-cause-unreadable = the raw file cannot be read
job-cause-edit = its edit cannot be read
job-cause-binding = an edited module cannot be applied to the photo
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

## An export's row in the jobs window: its details and buttons. $done is the photos
## written, $total those asked for, $failed those that failed, $file the photo being
## exported or a file written, $photo a photo's name, $folder where they go, $cause why a
## file could not be written, as the app words it, and $count the photos waiting. A single
## photo is the exact number 1, never the plural category one, which Ukrainian also gives
## 21.

# What the live region says as an export ends.
activity-name-finished =
    { $total ->
        [1] Export finished: 1 photo exported
       *[other] Export finished: { $total } photos exported
    }
activity-name-failed =
    { $total ->
        [1] Export failed
       *[other] Export finished, { $failed } failed: { $done } of { $total } photos exported
    }
activity-name-cancelled = Export cancelled: { $done } of { $total } photos exported
# How a row says an export ended.
activity-title-running =
    { $total ->
        [1] Exporting 1 photo
       *[other] Exporting { $total } photos
    }
activity-title-finished =
    { $total ->
        [1] 1 photo exported
       *[other] { $total } photos exported
    }
activity-title-failed =
    { $total ->
        [1] The photo could not be exported
       *[other] { $done } of { $total } photos exported
    }
activity-title-cancelled = Export cancelled
# A row's lines.
activity-now =
    { $total ->
        [1] Exporting { $file }
       *[other] { $done } of { $total } done · now { $file }
    }
activity-between = { $done } of { $total } done
activity-folder = To { $folder }
activity-failure = { $photo }: could not write { $file }: { $cause }.
activity-failure-photo = { $photo }: { $cause }.
activity-failure-cause = { $cause }.
activity-cancelled-detail =
    { $done ->
        [0] Nothing was exported before it stopped.
        [1] 1 of { $total } photos was exported before it stopped; it stays.
       *[other] { $done } of { $total } photos were exported before it stopped; they stay.
    }
activity-queued = Next in line: { photo-count }, to { $folder }
# A row's buttons: Show names the platform's file manager.
activity-show-finder = Show in Finder
activity-show-explorer = Show in Explorer
activity-show-files = Show in Files
activity-show-tip = Open the export folder
activity-cancel = Cancel export
activity-cancel-tip = Stop after the photo being written; the files written stay
activity-try-again = Try again
activity-try-again-tip = Export the photos that failed again, with the same settings
activity-dismiss = Dismiss

## The central bar, over the photo

central-bar = Central bar
photo-show-selected = Show all selected photos
photo-proof-margin = Proof margin
photo-before-after = Before and after
photo-guides = Guides
guides-show-label = Show guides
guides-show-full = Show guides
guides-show-promise = Show the guide overlay while this module has focus.
guides-settings-full = Change global guide settings
guides-settings-promise = These settings apply globally, to every module that shows guide overlays.
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
photo-zoom-tip = Zoom: pinch or { $command }+scroll; Z for 100%. Scroll moves a zoomed photo, Shift+scroll sideways
photo-zoom-tip-exact = At 100% the screen shows exactly what the export will be. Zoom: pinch or { $command }+scroll; Z for 100%. Scroll moves a zoomed photo, Shift+scroll sideways
# While the viewport can only fit the photo.
photo-zoom-later = Zoom comes with the 100% view
layer-image = Image

## The central panel

# While the photo decodes and its first frame renders; $name is its file name.
photo-opening = Opening { $name }…
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
# The exposure values at the end of the caption under the photo, from its EXIF. $value is
# written in the language's numbers: the shutter as "1/250" or "2.5", the aperture "8.0",
# the focal length "35", the ISO "200".
photo-shutter = { $value } s
photo-aperture = f/{ $value }
photo-focal-length = { $value } mm
photo-iso = ISO { $value }

## The platform's open dialog, from File › Open Image…

# Its title, where the platform shows one.
open-title = Open Image
# The file types it shows: the raws Apertrust can open.
open-filter = Raw photos

## The grid panel and its cells

grid-panel = Grid panel
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
# darktable's colour labels, in its order.
colour-label-red = Red
colour-label-yellow = Yellow
colour-label-green = Green
colour-label-blue = Blue
colour-label-purple = Purple
# The grid panel's edge towards the photo, which resizes it, for screen readers, and its
# value: how many rows of thumbnails it shows.
grid-panel-height = Grid panel height
grid-rows =
    { $count ->
        [one] { $count } row
       *[other] { $count } rows
    }
# At the left or right, the grid panel's edge resizes it in columns.
grid-panel-width = Grid panel width
grid-columns =
    { $count ->
        [one] { $count } column
       *[other] { $count } columns
    }
# The grid bar's options menu, a right-click on it, for screen readers, and its submenu of
# where the grid panel goes.
grid-bar-options = Grid panel options
grid-position = Position
# The grid bar, the header along the grid panel's top, as screen readers name it.
grid-bar = Grid bar
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
# Its Labels button, which shows the captions under the thumbnails or hides them.
grid-labels = Labels

## Folder mode: File › Open Folder…, the Library tab's Folders and the empty states

action-open-folder = Open Folder…
# The title of the platform's folder dialog, where it shows one.
open-folder-title = Open Folder
# The Library tab's sections. Albums and Smart albums come with the library.
section-folders = Folders
section-albums = Albums
section-smart-albums = Smart albums
# The open folder in Folders: its name and how many photos it holds.
folders-current = { $name } · { photo-count }
folders-reading = Reading { $name }…
folders-subfolders = Subfolders
folders-recent = Recent folders
# The tooltip of a recent folder that could not be opened when it was tried.
folders-recent-missing = This folder could not be opened. { $reason }
# The central panel and the grid panel with nothing open: where to start.
start-title = Open a folder to start
start-recent = Recent folders
folder-reading = Reading { $folder }…
folder-empty = No raw photos in { $folder }
folder-failed = Could not open { $folder }: { $reason }
folder-cancelled = Reading { $folder } was cancelled
# A folder is open with photos, and none is.
folder-no-photo = No photo selected
folder-choose = Choose one in the grid panel

## Settings

settings-close = Close
# Why a row is disabled: the command line set it for this run (decision 43). $flag is
# the flag, --conf, never translated.
settings-overridden = Set by { $flag } for this run
# Under a row whose setting takes effect only at the next start.
settings-after-restart = Applies after a restart
# What a row's dot, darktable's, tells screen readers: the setting is off its default.
settings-changed = Changed from its default
# The item of a row's right-click menu that puts its default back.
settings-reset = Reset
# The pages, listed at the dialog's start, and what a page that holds nothing yet says.
settings-page-general = General
settings-page-import = Import
settings-page-library = Library
settings-page-editing = Editing
settings-page-processing = Processing
settings-page-storage = Storage
settings-page-shortcuts = Shortcuts
settings-page-presets = Presets
settings-page-ai = AI
settings-page-later = Coming in a later version
# The titled groups of a page's rows.
settings-group-interface = Interface
settings-group-confirmations = Confirmations
settings-group-thumbnails = Thumbnails
settings-group-side-panel = Side panel
settings-group-gpu = GPU
settings-group-diagnostics = Diagnostics
# Each setting's row, by the setting's id, its dots and underscores as hyphens; a row's
# -help, where it has one, is its label's tooltip.
setting-theme-label = Theme
setting-theme-option-dark = Dark
setting-theme-option-light = Light
setting-theme-option-high-contrast = High contrast
setting-locale-label = Language
setting-scale-label = Interface size
setting-sliders-marker-label = Slider marker
setting-sliders-marker-option-triangle = Triangle
setting-sliders-marker-option-circle = Circle
setting-sliders-marker-option-square = Square
setting-export-ask-before-overwrite-label = Ask before overwriting an exported file
setting-grid-panel-square-thumbnails-label = Square thumbnails
setting-side-panel-tab-names-label = Show tab names
setting-engine-adapter-label = Graphics card
# What the graphics card's field shows while empty: the system's choice.
setting-engine-adapter-hint = Automatic
setting-engine-memory-budget-label = Memory budget
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

# The action, the top panel's mark that runs it, and the dialog's name for screen readers.
action-about = About { app-name }
# $version is the build's version, as 0.1.0+197~g9201df0.
about-version = Version { $version }
about-copyright = Copyright © 2026 { app-name } developers.
about-free-software = Free software under the GNU General Public License, version 3 or later.
about-credits = Credits
about-licence = Licence
# The button at the start of Credits and Licence that goes back to About.
about-back = Back

## Actions, as the keyboard map names them (docs/gui.md, Keyboard and actions)

# File › Open Image…: one raw, in place of the photo open.
action-open = Open Image…
action-view-edit = Switch to Edit
action-view-library = Switch to Library
# The side panel's tabs by position, whatever they hold.
action-panel-tab-1 = Side-panel tab 1
action-panel-tab-2 = Side-panel tab 2
action-panel-tab-3 = Side-panel tab 3
action-panel-tab-4 = Side-panel tab 4
action-panel-tab-5 = Side-panel tab 5
action-scale-up = Larger interface
action-scale-down = Smaller interface
action-scale-reset = Normal interface size
# F6 moves the focus between the top panel, the central bar, the side panel and the grid panel.
action-next-region = Next pane
action-previous-region = Previous pane
action-full-screen = Full screen
# File › Close Window on the Mac: the window with the focus, Settings' or the main window,
# which quits.
action-close-window = Close window
action-quit = Quit
action-peek = See the photo without the section under the pointer
action-next-photo = Next photo
action-previous-photo = Previous photo
action-zoom-in = Zoom in
action-zoom-out = Zoom out
action-zoom-fit = Zoom to fit
action-zoom-100 = Zoom to 100%
# The grid bar's menus, and View › Sort By and Thumbnail Size: the sort's key and
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
# Where the grid panel goes: the grid bar's options menu and View › Grid Panel Position.
action-grid-position-left = Left
action-grid-position-right = Right
action-grid-position-bottom = Bottom
# View › Show Labels and the grid bar's Labels: the captions under the thumbnails.
action-show-labels = Show labels
action-background-black = Black
action-background-very-dark = Very dark
action-background-dark = Dark
action-background-medium = Medium
action-background-light = Light
action-background-white = White
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
menu-photo = &Photo
menu-view = &View
menu-window = &Window
menu-help = &Help
# Submenus of View.
menu-panel-tab = Side Panel Tab
menu-interface-size = Interface Size
menu-sort-by = Sort By
menu-thumbnail-size = Thumbnail Size
menu-background = Background
menu-grid-position = Grid Panel Position
# The Mac's own items in the app menu and the Window menu, which macOS performs.
menu-services = Services
menu-hide-app = Hide { app-name }
menu-hide-others = Hide Others
menu-show-all = Show All
menu-minimise = Minimise
menu-zoom-window = Zoom
menu-bring-all-to-front = Bring All to Front

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

## The tone equalizer's graph of its bands; $module is the module's title

# The graph's full name, for screen readers.
equalizer-name = { $module }, curve
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

# The section that holds them, pinned at the top of the side panel's editing tabs.
section-scopes = Scopes
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

## The Export dialog (docs/gui.md, The Export dialog): darktable's export module, its
## labels in sentence case. $count is the photos, a single one the exact number 1. $file
## is a photo's name or a file's, $source the raw, $folder a folder, $width and $height
## pixels, $problem what stops the export.

export-title = Export { photo-count }
export-button = Export { photo-count }
export-close = Close
export-hint = Exports run in the background; you can keep working.
# The tooltip of every row and group drawn disabled because it is not built yet.
export-later = Coming in a later version

## The Presets column

export-presets = Presets
export-preset-default = Default settings
export-preset-built-in = Built in
export-preset-default-tip = The settings as they ship, darktable's defaults: choosing it resets them. It cannot be changed or deleted.
export-preset-matches-none = The settings match no preset.
export-presets-later = Presets of your own come in a later version.

## Storage options

export-storage-options = Storage options
export-target-storage = Target storage
export-storage-latex = LaTeX book template
export-storage-disk = File on disk
export-storage-email = Send as email
export-storage-gallery = Website gallery
export-path = Path
export-choose-folder = Choose…
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
export-conflict-one-number = { $file } is already there, so this one is saved with a number added.
export-conflict-one-overwrite = Replaces { $file }, which is already there.
export-conflict-one-skip = Skips { $file }, which is already there.
export-conflict-one-unchanged = Skips { $file }, which is already there and newer than the photo's edit.
export-conflict-many-number = { $count } of the { $total } are already there; those are saved with _01 added.
export-conflict-many-overwrite = Replaces the { $count } of the { $total } already there.
export-conflict-many-skip = Skips the { $count } of the { $total } already there.

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
export-size-capped = { $file } will be { $width } × { $height } px (not upscaled: Allow upscaling is No).
export-size-full = { $file } will be { $width } × { $height } px (full size).
export-size-no-limit = { $file } will be { $width } × { $height } px (full size: no limit set).
export-upscale = Allow upscaling
export-high-quality = High quality resampling
export-high-quality-tip = Do high quality resampling during export: every operation at full size, then scaled down.
export-no = No
export-yes = Yes
export-masks = Store masks
export-masks-later = Store masks as layers in exported images. Only TIFF, OpenEXR and XCF can, and layers come with a later version.
export-profile = Profile
export-profile-image = Image settings
export-profile-tip = Image settings: the photo's own, in Colour › Output colour profile, as the view shows it. Another profile applies to this export alone.
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
