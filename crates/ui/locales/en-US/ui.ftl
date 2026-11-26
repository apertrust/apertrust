# Apertrust's interface in American English (docs/gui.md, Translation): an overlay of
# en-GB/ui.ftl, not a copy. It holds only the messages whose words American English spells
# differently (color, gray, center, minimize, canceled, license), with en-GB's variables;
# every other message is en-GB's.

## The Tools panel's tabs

tab-color = Color
panel-customise-tabs = Customize tabs…

## The Styles tool

style-builtin-natural-colour = Natural color
style-builtin-vibrant-colour = Vibrant color

## Themes

theme-grey = Gray

## The Toolbar

window-minimise = Minimize
window-maximise = Maximize
action-customise-toolbar = Customize toolbar…
toolbar-customise-later = Customize Toolbar comes in a later version

## The jobs

jobs-cancelled = Canceled: { $message }
job-export-cancelled =
    { $total ->
        [1] Export canceled
       *[other] Export canceled: { $written } of { $total } photos exported
    }
job-cache-clear-cancelled = Clearing canceled: { job-cache-entries } removed, { job-cache-size }

## The viewer bar

photo-values-read = Red { $red }, green { $green }, blue { $blue }, gray { $grey }

## The Browser and its cells

grid-cell-labels = Color labels: { $labels }
grid-cell-colour-labels = Color labels

## Folder mode

folder-cancelled = Reading { $folder } was canceled

## Settings

setting-theme-option-grey = Gray

## About Apertrust

about-licence = License

## The menus' titles

menu-colour-label = Color Label
menu-minimise = Minimize

## The scopes

scope-vectorscope-tip = Colors by hue, the angle, and by saturation, the distance from the center.

## Colour pickers

picker-tip = Pick color from the photo
picker-patch-input-tip = The input color that should be mapped to the target.

## Format options

export-jpeg-subsample-tip = Chroma subsampling for the JPEG encoder. Auto: from the quality. 4:4:4: none. 4:4:0: color halved vertically. 4:2:2: halved horizontally. 4:2:0: halved both ways.

## Global options

export-size-cm = Size in centimeters
export-profile-tip = Image settings: the photo's own, in Raw › Output color profile, as the view shows it. Another profile applies to this export alone.
