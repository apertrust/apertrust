# 0042. Numbers shown, typed and stored

## Context

How does a slider show its value, how is a number typed, how is fine control given, and how do choices look in a module panel? darktable's users know its bauhaus sliders and comboboxes (`bauhaus.c`), and the first pass mirrors darktable ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Decision

**At rest a slider's value is read-only, written as darktable writes it** (`dt_bauhaus_slider_get_text`): with its unit (`unit-<unit>-value`, Ukrainian and Polish as darktable's translations write them, "50%" and "px"), signed whenever the hard range spans zero, zero as "+0.00", CLDR digits with U+2212, in 12-point text, right-aligned 5 points from the row's end, never cut; the label is cut instead.

**A click on the value, or a typed digit, minus sign or separator, opens a field** in its place (`ValueField`), the number alone and selected, as wide as it needs. The field takes a number only (`ValueField::number_only`): digits, a leading minus (`-` or U+2212) and one decimal separator, the point or the language's own; any other character is ignored as typed; a paste or a screen reader's set value is taken whole or refused whole. Enter, Tab or a click elsewhere sets it, rounded to the row's decimals and clamped to the hard range, the track widening past the soft range to show it; Escape puts it back; empty, or a sign alone, puts the value back with no error. The field is named with the slider's unit for screen readers ("Exposure, in EV"). darktable's expressions wait until there is a need: `apertrust_ui::calculator`, darktable's `dt_calculator_solve` restated over f64, is built and unused.

**darktable's fine-control popup, nothing removed,** opens on Enter, a right-click, Shift+F10, the Menu key or AccessKit's ShowContextMenu, drawn on every platform as a kind of the popup service ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)), with darktable's geometry, fan, pick-up rule, precision, zoom and keys and the approved deviations listed in [gui.md](../gui.md) (planned).

**A cancelled gesture leaves no trace:** `Command::CancelGesture { module }` puts the module back as it was before the gesture's first `SetControl { done: false }`: values, marks, follow writes, clamps, the held view coordinates and a switched module's on state. It makes no history step and writes nothing; the session reports `Saved` with the cancel's number. Any other command on the edit ends the gesture as completed; a resize, the scopes and a cancel naming another module leave it under way.

**Choices have two styles** (`Choice::style`): `Blended`, darktable's combobox, in module choice and preset rows (the option right-aligned with ⌄ after it, no box, the hover band across the row, the kept marker a slider row has), and `Boxed`, the default, everywhere else. An option the photo does not list is shown last, disabled and described ([0030](0030-controls-as-darktables.md)).

The curve's readout and the equalizer's graph have no value field and no popup.

## Options considered

- A value field always shown with the number alone: unlike darktable, and noisier at rest.
- Enter moving to the field: darktable's Enter opens the popup.
- The client sending the old value and mark back on a cancel: it leaves follow writes, clamps and a switched module on, and makes history steps of nothing.
- No live values in the popup: darktable shows them.
- Filtering a paste to number characters: it changes a pasted number unseen.
- Expressions in the field now: no need yet.

## Consequences

- `SliderRow` takes its sign rule from its range; `format_number` writes a signed zero with "+".
- The session holds a gesture's starting state per module until it ends.
- Planned: the popup's pointer, keys, zoom and middle click; the colour wheel's rows.
