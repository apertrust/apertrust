# 0032. Tools off until first touched

## Context

darktable leaves most modules off on a new photo, and turning one on applies its defaults, which change the image: grain 25 %, soften's mix 50 %, vignetting's brightness and saturation −0.5, haze removal 0.2, local contrast's detail 125 %. Some operations, bloom and denoise among them, have no setting that leaves the image unchanged at all. How does a module that darktable leaves off start, turn on and go off again, and how does that look, when the first pass mirrors darktable ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)) and a module that is off must change nothing?

## Decision

**A module darktable leaves off on a new photo is switched** (`Definition::switched`): off until a command writes one of its controls, then on with darktable's defaults for every control not set, until its reset turns it off. **A module that is off is a no-op**, covered by an invariant test. A module whose operation every photo runs is always on, untouched at its defaults ([0028](0028-tools-bound-to-operations.md)); `check::accept` refuses a switched module over such an operation.

**From darktable:** `dt_iop_gui_changed` enables a module on every slider and combobox callback, without comparing values, so a click at the same value turns an off module on; its history stores `enabled` with the parameters.

**The edit:** `ModuleValues::enabled`, the module's own state beside its values and marks, written in `.apxmp` version 1 as `apertrust:enabled="true"` on an on module's entry, absent meaning off. An off module holds no value off its defaults; reading repairs one that does, as on with a warning. `enabled` on a module this build does not switch makes the photo view-only.

**Planning:** `edit::instances` plans an optional operation while a switched module owning one of its parameters is on. It is bound with its values off its defaults, possibly none, so the operation runs at its own defaults, which `check::switched_defaults` holds equal to the module's: on at darktable's defaults is darktable's new instance.

**Commands:**

- Any write of a control turns a switched module on, whatever the value: `SetControl` on a control or a view's coordinate, `ApplyPreset`, their follow writes, and Auto and pickers when they come; a picker turned on turns its module on as darktable's does, before any write ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)). A script turns a module on at its defaults by setting a control to its default.
- `ResetControl` and `Keep` never turn one on, and leave an on module on.
- `ResetModule` clears its values and marks and turns it off; `Reset` turns every module off.

**Reported:** `ModuleState::enabled`, true for a module not switched, but while the history's end lies before its default step and darktable leaves it off until its workflow applies it ([0078](0078-undo-redo-and-the-edits-history.md)), and for a switched one while on; `ModuleState::edited`, whether its reset would change the render; `Values::edited` counting an on switched module.

**The panel shows nothing more:** no dimming and no power button. The section looks as usual at darktable's defaults; the kept marker beside a changed control and the header's reset, available only while the module is edited or a change is pending, are the only cues. Screen readers hear "Not applied" in the section's description while it is off (`section-not-applied`).

**Rows every switched module's table has:** off renders as no edit, exactly; on at darktable's defaults binds the operation's defaults exactly; the first step, the module on at darktable's defaults, measured and reported without a bound; a move then `ResetModule` renders as no edit, a move then `ResetControl` as on at darktable's defaults; an older edit migrates to the parameters it bound. No row at zero.

**Old edits:** a module that becomes switched raises its version, and its migration turns an old edit's module on where a value was off the old defaults, writing those defaults where darktable's differ, and drops the values of one that rendered nothing, so each edit binds what it bound.

**Differences from darktable that remain:** a reset turns a module off, where darktable's leaves it on; no module is off with its values kept, or on at its defaults from the panel alone; the modules every photo runs cannot be turned off, the history's end before a default step leaving one not applied only as darktable's history shows it (0078).

## Options considered

- A least-effect value as the untouched state, darktable's defaults a preset: bloom's strength 0 still blooms once another control moves, and denoise's 0 is out of range.
- An `on` control in no row: every list, mark and `--set` would show it, and it allows an off module with values.
- On while the edit holds a value: on at its defaults and off would differ only by a value written at its default.
- Switched derived from the manifests: switches every module at once.
- Turning on only for a value that moves: darktable does not, and a script could not turn a module on at its defaults.
- Dimming an off module's controls, or darktable's power button.

## Consequences

- Built: Grain, Soften, Vignetting and Haze removal at darktable's defaults, each version 2 with its migration; Colour balance RGB, RGB primaries and the Tone equalizer switched at the defaults they have, version 2 too; the panel's reset and description; a test holding the switched modules to those darktable leaves off, every module over optional operations but the Tone curve, which is to become RGB curve. Planned: `apertrust-cli edit --reset MODULE`.
- Colour balance rgb, as in darktable, runs only while its module is on, so an unedited photo has no gamut mapping darktable's lacks; its first step brings that mapping, measured and reported.
