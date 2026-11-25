# 0029. A tool is darktable's page

## Context

What does a first-pass tool show? darktable's modules have tabs, collapsible sections, rows that appear only in one mode, pickers and presets, and many controls whose processing Apertrust has not ported yet. Showing only what is ported would change the panel as ports land and hide from a darktable user what is missing; a Basic/Advanced split of Apertrust's own would not be darktable's page.

## Decision

**Each tool is darktable's page as darktable has it,** under darktable's name, from its `gui_init`, `gui_changed` and `gui_update` at the reference release. Exposure's page is the one exception, redesigned ([0088](0088-exposures-page-as-redesigned.md), planned).

- **Every control darktable shows,** in darktable's order, under its labels and section labels, ported or not. A control not ported is drawn disabled and takes only its default (`Control::unported`). A choice lists every option darktable lists on some photo ([0030](0030-controls-as-darktables.md)), those not ported drawn disabled and refused as commands.
- **Every darktable tab is a content tab,** in darktable's order, masking, masks, options and advanced included: the tone equalizer's Simple, Advanced and Masking; colour balance rgb's Master, 4 ways and Masks; filmic rgb's Scene, Reconstruct, Look, Display and Options. Rows darktable shows under every tab show whichever tab is chosen. A titled tab without rows yet is drawn disabled.
- **There is no Advanced controls toggle** and no Basic/Advanced split.
- **darktable's section labels are plain groups and its collapsible sections collapsible groups,** the two kinds of one `Row::Group` holding their rows ([0100](0100-groups-plain-and-collapsible-one-parent.md)). A plain group is always open. A collapsible group is at darktable's place and under its label, closed at first (the Export tool's Global options excepted, open at first since the size is the next thing an export depends on, planned, [0090](0090-export-as-a-photos-tab-tool.md)), its open state kept app-wide in settings, never in the edit; it holds no other collapsible group, and a plain group holds no group. A closed group shows a dot when a control in it is set; to AccessKit it is a button with `expanded` and "N set".
- **Rows follow darktable's conditions** (`Row::When`): a row darktable shows only in one mode, or while another control holds a value, is hidden otherwise. A control that cannot act in the current state is drawn disabled, never hidden, but for Exposure's two camera rows, Compensate camera exposure on a photo with no bias and Highlight preservation mode where the camera held nothing back, which `ProcessingTool::visible` hides ([0088](0088-exposures-page-as-redesigned.md)); hiding any other row needs a decision of its own. One parameter shown under two labels by mode has one row per mode, one shown at a time (`Slider::label`).
- **Quads, pickers and wands** are darktable's: a picker on a row, a showmask view mode, an Auto quad for one control. Each is an action that writes controls, a picker that writes only settings, as Exposure's Reference picker ([0088](0088-exposures-page-as-redesigned.md)), or a view, and none is stored; actions not yet built are drawn disabled. darktable's packed lines are one line row with end items, a slider's quad its end item ([0107](0107-packed-lines-one-line-row-with-end-items.md); built, the line in the three shapes it replaced and a slider's picker). Readouts and read-only graphs are drawn as darktable shows them.
- **darktable's own settings for a tool**, kept in darktablerc and never in an edit, are rows of no control (`Row::Setting`), kept in Apertrust's settings once what uses them is built. A control darktable shows only under a preference that is off by default, or only for an edit made with an older algorithm, is not shown; its parameter stays at darktable's default or current value.
- **Presets are darktable's,** from `init_presets`, in the header's presets menu: applying one is one change and one history step; darktable's auto-applied workflow presets are Apertrust's defaults already; a preset that sets a fixed parameter or a mode not ported is left out.

**Visibility and disabled options are the renderer's,** from the definition and the mirrored values, so neither the edit nor the render depends on them. `ProcessingDefinition::check` proves that each control appears in one row, or one row per mode, or only in the rows of views covering it ([0031](0031-views-and-curves-of-the-same-stored-values.md)).

## Options considered

- darktable's sections behind an Advanced toggle, or always open: not darktable's page, or a long panel.
- Only what is ported: changes the panel as ports land and hides what is missing.
- Unported rows hidden: against "what you see is true".
- Every mode's rows always shown, disabled out of mode: clutter darktable does not have.
- Tabs or groups of Apertrust's own: not darktable's page.

## Consequences

- [tools/README.md](../tools/README.md) holds the rules every tool page cites; each built tool's page lists its rows, defaults and invariants.
- A darktable user finds every control where darktable has it, and sees what is still to come.
