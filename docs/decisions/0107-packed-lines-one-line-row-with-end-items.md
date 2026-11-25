# 0107. Packed lines: one line row with end items

## Context

darktable packs several widgets on one line in many of the first-pass modules ([0029](0029-a-tool-is-darktables-page.md)). At the reference release ([0003](0003-the-reference-is-darktables-latest-stable-release.md)) there are 30 such box lines, 38 quads (a button at the end of a slider or a combobox, `src/bauhaus/bauhaus.c:1111-1121`), and the utilities' lines. Fourteen modules have none.

What sits at a line's end is one of seven things:

- a colour picker;
- a pure action on the tool's own values;
- an action that reads another tool;
- a computation the engine makes from the photo;
- a mode on the photo;
- a view of a mask;
- a read-only warning.

Ctrl and Shift pick variants of some actions. Availability and mutual exclusion are the only rules the lines carry.

A definition has five shapes for these today: `Row::Pickers`, `Row::Buttons`, `Row::Action`, a slider's `Quad` (`Picker` or `Line`) and custom widgets. They split darktable's lines where they meet. AgX's Auto tune levels and its Read exposure camera button share one line in darktable (`src/iop/agx.c:2068-2084`) but are two rows here. Its Set from above shares Reverse all's line (`agx.c:2339-2356`) but is a row of its own. Showmask views, computing quads and warnings have no shape at all. `Row::Action` draws a button that never runs, and an action sees only its own tool's values, which AgX's Read exposure cannot work with.

## Decision

**One line row replaces `Row::Pickers`, `Row::Buttons` and `Row::Action`, and an `end` list of a closed set of end items replaces the slider's quad.** The line row is `Row::Line { start, readout, end }`. Slider, Choice, Toggle and Setting rows carry an `end` list too. Every part is planned; nothing is built by this decision. [gui.md](../gui.md#line-rows-and-end-items) holds the details.

1. **The line row.** `Row::Line { start, readout, end }`:
   - `start` is nothing, a label (a plain group's words, as `Row::Pickers`' label is), or a control (its label being darktable's reset label, double-clicked to reset it, as `Row::Buttons`' is).
   - `readout` is a text and a swatch the tool computes from its values, in the label's place, as Colour calibration's CCT is (`ReadoutDef`).
   - `end` is the row's end items.

   A slider's `Quad` becomes its `end`: one item, since darktable's quad is one. A choice, a toggle or a setting may take a few. The end items are drawn where a row's buttons go today, before the value, the option or the switch. A line with no start and no readout whose one item is a text `Action` draws it at the start, where `Row::Action`'s button is today.

   This lands first, in a change of its own with no change of behaviour, before any new kind of end item: every page reads as it does now.
2. **End items, a closed list** (`End`):

   | Item | What it does | darktable's |
   | --- | --- | --- |
   | `Picker(id)` | Turns a picker of `ProcessingDefinition::pickers` on and off ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)), as `Quad::Picker` and `Row::Pickers`' buttons do. A picker declared one-shot (`Picker::once`) is never left on: a press sends `PickerCommand::Once` over the whole photo (79.17), as RGB levels' Auto | 17 quads; the pickers on AgX's, Colour calibration's, RGB curve's and RGB levels' lines; RGB levels' Auto |
   | `Action` | A pure action, `ProcessingTool::action` on the values shown and the photo, one change. Its face is an icon or a text. It may open a menu of several actions in place of running one, as AgX's Reset primaries does (`agx.c:2280-2288`, `:2100-2121`) | Orientation's four buttons, AgX's three, crop's aspect flip, lens correction's scale to 1 |
   | `Compute` | A result the tool's operation computes from the photo, which the tool's `apply` writes as one change (rule 4) | The tone equalizer's wands, demosaic's capture sharpen radius, Rotate and perspective's detect lines and fit, the colour checker's validate and recompute |
   | `Mode` | Chooses a cursor of the tool's ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)), its button a second door to it (rule 7) | Rotate and perspective's structure lines and rectangle; straightening |
   | `View` | Shows a mask on the photo in place of the render, which is view state (rule 6) | 13 showmask quads; Filmic RGB's and the tone equalizer's mask toggles |
   | `Warning` | A read-only sign shown while the tool's check reports it (rule 9) | AgX's toe and shoulder power quads (`agx.c:1727-1735`) |

   **Availability** follows the tool's `available` rule, extended to end items. It is pure, on the values shown and the photo, reported by the session with the tool's state, and followed at once by the app's pending display, as a control's is. An item that cannot act is drawn disabled. AgX's Set from above is insensitive while Reverse all is on (`agx.c:1744-1746`), and Rotate and perspective's fit is insensitive until there is a structure to fit. A picker's availability stays `ProcessingTool::picker_ready`.
3. **Published facts between tools.** An action may declare that it reads a named fact another tool publishes. The publishing tool computes the fact with its own pure function of its values and the photo, and declares it in its definition. The action receives that value alone, never the whole edit's values. Wherever the action runs, in the app or for a script, the fact comes from the same edit, through the same function.

   The first is Exposure's `effective_exposure`: its exposure, less the camera's bias where Compensate camera exposure is on, plus the held-back exposure where Highlight preservation mode is on, in Automatic mode too, where darktable's value is the stored one, cached before its automatic exposure is worked out. That is darktable's `dt_dev_exposure_get_effective_exposure` (`src/develop/develop.c:3387`, through Exposure's proxy, `src/iop/exposure.c:1328-1335`, cached in `commit_params`, `:604-634`). AgX's Read exposure sets the range from it (`agx.c:1042-1052`).

   The type is `Published`, kept apart from the header tooltip's `Fact` and the events' facts ([0059](0059-one-event-system.md)).
4. **`Compute` is shaped like a pick.**
   - The session asks the tool's operation for a named result on the current render, as `Operation::computed` gives a computed value ([0087](0087-the-raws-histogram-counted-once-for-an-operation-that-reads-it.md)).
   - The tool's pure `apply` turns the result and the tool's values into the controls it writes, one change and one history step, as a pick's reading is applied.
   - What darktable computes that a picker can do is a picker. RGB levels' Auto is a one-shot pick over the whole photo, its button a `Picker` end item of a picker declared one-shot, sending `PickerCommand::Once` (79.17), and its Auto region an area picker (`rgblevels.c:1081-1097`).
   - What needs no pixels is an action. Lens correction's autoscale is a pure action given the lens data and the photo's size (`lens.cc:4211-4219`).
   - Each compute comes with its tool's port.
5. **Modifier variants are actions of their own.** Each variant has its own id, as `tool-ashift-fit_vertical_rotation` beside `tool-ashift-fit_vertical`.
   - A click on the button runs its plain action. Ctrl (Cmd on the Mac) or Shift runs the variant, as darktable's modifiers do (`ashift.c:5334-5341`, `:5482-5492`). The button's tooltip lists the keys.
   - Every variant is reachable without a modifier: on touch ([0094](0094-one-interface-standard-and-touch-modes.md) settles where), by shortcuts, by scripts and through the MCP. Each is listed among [0098](0098-actions-and-shortcuts-after-darktable.md)'s actions, as History's Compress and Truncate already are, separately.
   - A gesture's keys on the photo stay the gesture's, as RGB curve's create-curve drag reads Ctrl and Shift through `PickKeys` (`rgbcurve.c:1513-1517`).
6. **One view across the edit.** A `View` is view state, as `Hide` is ([0078](0078-undo-redo-and-the-edits-history.md)): never stored, no history step, never exported.
   - Turning one on turns any other off.
   - It turns off when pressed again, when another view turns on, on Esc, when the photo changes, and when its tool is reset or turned off.
   - While it is on, a label on the photo names it, as a picker's area names its tool.
   - Turning it on sends a session command of its own beside `Hide`, never on the edit, and the session reports which view is on.
   - So darktable's rule that Colour balance RGB's mask is refused while the blend mask shows (`colorbalancergb.c:1421-1425`) is the one-view rule.
   - It is built with the first tool that has a view.
7. **Every mode on the photo is a cursor** ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)).
   - Its `Mode` button is a second door to the cursor, as a picker's button is to its picker cursor: pressing it chooses the cursor, and the cursor chosen lights the button. One cursor at a time makes modes exclusive with no declaration.
   - `Quad::Line` becomes a `Mode` at the end of Rotate and perspective's rotation slider, choosing a Straighten cursor.
   - A Geometry cursor group comes with those ports: Straighten, Rotate and perspective's structure lines and rectangle, and crop's frame.
   - A tool's main modes are in the well. Its advanced ones are reached from their button alone, as [0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md) rules for pickers.
   - A Geometry cursor decides what a press on the photo does. A tool's Show on photo toggle, planned with Rotate and perspective's port ([0098](0098-actions-and-shortcuts-after-darktable.md)), decides whether its controls on the photo are drawn. How the two meet, whether choosing the Crop cursor turns Crop's Show on photo on, is settled with those ports.
8. **Two layouts besides the line:**
   - **`Row::Fields`** puts two or three compact controls, numbers or choices, on one line, sharing its width. Each field may have a separator before it (`×`, `@`) and a unit after it. The line has no label of its own: each control keeps its own accessible name, tooltip and actions. Below the width they need, the fields fold into ordinary labelled rows, one each.
     - Lens correction's focal length, aperture and distance use it (`lens.cc:4406-4432`). Whether their choices take typed values, as darktable's editable comboboxes do, is a question for its port.
     - Export's pixels line moves to it, `[w] × [h] px`, its fields the width and height settings. The centimetre and inch lines, `[w] × [h] cm @ [dpi] dpi`, show lengths worked out from the stored pixels and the resolution, which a field naming one setting cannot show, so they and the scale stay `Widget::ExportSize`. The pixels line's fields fit at their least width across the Tools panel's 240 to 600 points, so it stays one line with no label there, as [0090](0090-export-as-a-photos-tab-tool.md) has it. A utility's fields name its settings by their keys, as its setting rows do ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)).
   - **A tab strip may end with end items,** pickers or actions, after the last tab: RGB curve's Show colour and Create curve at the end of its channel tabs (`rgbcurve.c:1485-1560`).
9. **`Warning`** sits in the quad's place, before a slider's value.
   - It shows only while the tool's pure check, on its values and the photo, reports it, in the `warning` colour. Its message is its tooltip and its description to screen readers.
   - Its place stays reserved while it is hidden, so the value never moves. It has no press action.
   - AgX's toe and shoulder power warnings are the first: the curve has lost its S shape and the fallback curve takes over (`agx.c:1727-1735`, tooltips `:1856-1880`).
   - darktable's preference to hide them (`plugins/darkroom/agx/enable_curve_warnings`, on by default) is not offered.
10. **No colour settings now.** Colour balance RGB's mask view draws a fixed checkerboard from the theme in place of darktable's two checkerboard colour settings (`colorbalancergb.c:2034-2049`). A colour kind of setting, a swatch opening a colour chooser, is planned for these and later needs.
11. **Not covered by a kind:** the colour checker's chart frame on the photo, which darktable draws while its section is open (`channelmixerrgb.c:2897`). It is decided with that port.

**The order of the work:**

1. `Row::Line` and `end` replace `Row::Pickers`, `Row::Buttons`, `Row::Action` and `Quad`, with no change of behaviour.
2. AgX's buttons:
   - a press runs `ProcessingTool::action`;
   - end items with their availability;
   - Reset primaries with its menu, and Set from above at the end of Reverse all's row;
   - Read exposure through Exposure's published `effective_exposure`;
   - the toe and shoulder warnings;
   - Auto tune levels and Read exposure back on one line, as darktable has them.
3. `Row::Fields`, with Export's pixels line moved to it.
4. The rest with each tool's port: `Compute`, `Mode` and the Geometry group, `View`, the tab strip's end items and the colour kind of setting.

## Options considered

- **Keep the three rows and add new kinds beside them:** every rule written for several rows that do the same thing, and AgX's line still split in two.
- **A free-form box of any widgets:** darktable's `dt_gui_hbox` as data. Nothing to check, every layout a new case for the renderer, and the accessibility and action rules left to each tool.
- **The whole edit's values given to an action:** makes every action depend on every tool, which neither a test nor layers could hold apart. One named fact keeps the dependency declared and the action pure.
- **One action with a modifier mask, as `pick` takes `PickKeys`:** a variant then has no id, so no shortcut, script, MCP tool or touch route reaches it.
- **Views per tool, exclusive only within one:** two tools' masks could show at once over one photo, and darktable's blend-mask refusal would need a rule of its own.
- **Modes as a state of their own, outside cursors:** a second "what a press does", beside the well, that could disagree with it.
- **Lens correction's three fields stacked, and RGB curve's pickers on a line under the tabs:** each departs from darktable's page with no gain, and Export's pixels line would stay a custom widget.
- **Warnings in a channel under the header,** as Last pick's warning lines: the warning then sits away from the slider it is about, and a line of text in the panel, which only important warnings get.
- **A colour setting now:** a new setting kind and a colour chooser for two settings that a fixed checkerboard serves.

## Consequences

- **`apertrust-tools`:**
  - `Row::Line`, `LineStart` and `End` replace `Row::Pickers`, `Row::Buttons`, `Row::Action`, `Quad` and `Button`; Slider, Choice, Toggle and Setting gain `end`.
  - The checks carry over: each picker in one place, a label never beside a readout, a line's label id unique among the tool's groups, `tabs` refused as an action id.
  - Later, in order: end items' availability; `Published` with `ProcessingDefinition::publishes`, `ProcessingTool::publish` and an action's `reads`; `ProcessingTool::warning`; `Row::Fields`; `Compute` with `ProcessingTool::apply`; `View`; mode cursors in `ToolDefinition::cursors` and `CursorGroup::Geometry`; a tab strip's end items; `Picker::once`, with RGB levels.
- **The engine:** the published facts given to an action; the compute command and the view command with its fact, each with its first tool.
- **`apertrust-ui`:** a line row drawing a start and end items; the warning sign in a slider's quad place; the fields row and its folding; buttons at a tab strip's end.
- **The app:** one drawing path for end items; the generated actions and their kinds (decisions [0091](0091-context-menus-places-and-tools-items-as-actions.md) and [0098](0098-actions-and-shortcuts-after-darktable.md)) read from end items, each variant an action of its own.
- **AgX's page** ([tools/agx.md](../tools/agx.md)) gets its line back and its three actions and two warnings.
- **Departures from darktable** ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)):
  - one view across the edit, where darktable keeps one per module;
  - variants as actions of their own, as well as by their modifier;
  - no preference hiding AgX's warnings;
  - a fixed checkerboard for the mask view.

  Each tool's page lists those that touch it.
- **The documents:** [gui.md](../gui.md#line-rows-and-end-items) holds the rows and items, [tools/README.md](../tools/README.md) rule 10 the rule, [pickers.md](../pickers.md) a picker's place, [cursors.md](../cursors.md) the mode cursors and the Geometry group, [settings.md](../settings.md) the colour kind planned, and [roadmap.md](../roadmap.md#line-rows-and-end-items) the steps.
