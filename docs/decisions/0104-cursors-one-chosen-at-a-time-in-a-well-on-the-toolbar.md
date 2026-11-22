# 0104. Cursors: one chosen at a time, in a well on the Toolbar

## Context

A press on the photo can mean many things: pan a zoomed photo, pick a neutral area for White balance, set Exposure from an area, paint a mask, heal a spot, mark the photo with a note, apply one photo's edit to another. darktable at the reference release has no shared way to choose between them: its pickers are turned on from each module's picker button, its masks drawn from the mask manager and each module's drawn-mask buttons, and a drag pans unless a module takes it. A quick pick could also be an item in the photo's right-click menu, "Set White Balance from Here", which needs a pick at one point as one completed change, an open question of [0091](0091-context-menus-places-and-tools-items-as-actions.md).

The question is how a person chooses what a press on the photo does, for pickers now and masks, retouch, annotations and copying edits later, with one API every owner uses, without changing darktable's own picker button and without the engine learning anything new. It is product scope darktable does not cover ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), on the Toolbar ([0092](0092-the-toolbar.md)), with keys after darktable ([0098](0098-actions-and-shortcuts-after-darktable.md)) and the picker commands of [0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md).

## Decision

**One cursor is chosen at a time; it decides what a press on the photo does, and it stays chosen until another is chosen.** The cursors sit in one sunken well on the Toolbar, one button per group. Cursors are an additional speed-up, never the single way to a feature: tools keep their own buttons. The design in full, with its measures, words, keys, API and plan, is [cursors.md](../cursors.md). Built in Phase 2: the one-shot pick (104.1), the tools' declarations with White balance's and Exposure's picker cursors (104.2), the well in the design system (104.3) and the app's Select, Pan and Pick (White balance, Exposure) on the Toolbar (104.4); the rest comes with its feature.

### The groups

In the well's order, each a group of its own even with one cursor:

| Group | Cursors | A press | When |
| --- | --- | --- | --- |
| Select | Select, the default at every start and Esc's target | A click selects a mark on the photo; a drag that starts on a mark moves it, one that starts on empty photo pans a zoomed photo, as darktable's drag does | Phase 2; marks to select come with stored samples and masks |
| Pan | Pan | A drag always moves a zoomed photo, even over a mark | Phase 2 |
| Pick | White balance, Exposure; Colour readout later | A click picks the area darktable's click gives the picker, a drag the area drawn: one `PickerCommand::Once`, one step where it changes the edit | Phase 2; Colour readout with the Colour picker (79.16) |
| Mask | Brush, Linear gradient, Radial gradient, AI object, Path | The chosen layer's mask, one step a stroke or shape ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)) | Phase 4 |
| Retouch | Heal, Clone | Spots on a retouch layer | Phase 4 |
| Annotate | Pen, Arrow, Text | A note on the photo | Later, its own decision |
| Copy and apply | Copy and apply | A click on a photo, in the Viewer or on a thumbnail, applies the copied edit; Alt-click copies | Phase 3, with Copy Parts |

- **Scroll and the trackpad pan** a zoomed photo with every cursor, and hold A pans from any cursor, darktable's own key. Scroll with a mask or retouch cursor is provisional until masks are built.
- **What a cursor does is in its tooltip,** "White balance picker · K — click a neutral area, or drag one"; the viewer bar carries no help line. The pointer takes the cursor's shape over the photo.
- **A cursor that cannot act** (the photo not live or open for viewing only, a tool hidden by the History tab's eye, Exposure's picker not ready) stays chosen, its accent kept: a press does nothing, the pointer over the photo is the not-allowed one and the tooltip says why. With no photo shown the whole well is disabled, the chosen face dimming with it, as any disabled control.

### The well

- **One toolbar item, `cursors`,** a part of the Toolbar's ([0092](0092-the-toolbar.md)), centred between two flexible spaces after Apply edit: `… copy-edit, apply-edit, flexible-space, cursors, flexible-space, jobs, assistant, save-state`. It never folds; the actions fold before it. Before decision 92's list is built it is a fixed part in the same place.
- **A `sunken` track** 30 points high, `radius_large`, 2 by 4 points of padding, its buttons `spacing.s` apart; client area, so a double-click on it never maximises the window.
- **One button per group, two targets, no plates** at rest, under the pointer or chosen ([0081](0081-icon-only-buttons-draw-no-plates-on-is-the-filled-icon.md)): the face, 22 by 26 points with its 16-point icon at its end, chooses the group's active cursor; the chevron, 11 by 26 right against it, opens the group's list. A group of one with no settings, Select and Pan, is one face 24 points wide with no chevron.
- **The chosen cursor's icon is in `accent_text`,** its stroke a quarter heavier than the outline (`STROKE_HEAVY`), and keeps that look while it cannot act now, its reason as tooltip, but dims with the whole well while no photo is shown; under the pointer an icon lifts to `text_strong`; a cursor not chosen that cannot act now is drawn disabled, `text_disabled`; the focus ring on one target.
- **The list** (the popup service, [0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)): the group's cursors with their keys, a check on the active one, choosing one making it active and chosen; then, under a line, the group's own settings, toggles and choices its cursors share, app settings in `settings.toml`'s `[cursors.<group>]`, never history. Pick's first is Show the last pick on the photo (`cursors.pick.show_last`), on by default, Phase 2: the area a picker cursor last read stays drawn as a stored area is until the next pick, another photo or another cursor. Each later group's settings are chosen with that group: Copy and apply's in Phase 3, Mask's and Retouch's with Phase 4, Annotate's with its decision.
- **Amounts** (a brush's size, hardness, flow) are no list items: a second click on the chosen face opens them as slider rows, a 4-point triangle marking a face whose cursor has them; `[` and `]` change the size.

### Keys

| Cursor | Default |
| --- | --- |
| Select | Esc, where nothing focused keeps it |
| Pan | A, held |
| White balance | K |
| Exposure | None |
| Mask and retouch cursors | Chosen with Phase 4 |

- **A cursor's key, tapped, chooses it and it stays,** but Pan's A, which is a hold alone, as H's peek: Pan is chosen while A is held, never kept. Held, a cursor's key borrows the cursor until released, the one before returning (Phase 3, with decision 98's dispatcher; Phase 2 has taps). Space stays Next Photo.
- **Each cursor is an action** of the `hold` kind ([0098](0098-actions-and-shortcuts-after-darktable.md)): `on` chooses it, `hold` borrows it, `off` chooses Select, `toggle` chooses it or, chosen, Select. A tool's is `tool-<tool>-cursor-<cursor>`, the others' `cursor-<id>`.

### Tools keep their buttons

- **A tool's picker button is darktable's, unchanged:** on, live readings as the area moves, off. While a tool's picker is on it takes the photo, and the chosen cursor waits underneath until it turns off.
- **The Layers section's mask and retouch buttons stay** (Phase 4) and share one state with the cursors: pressing a mask button chooses its cursor, which the well shows, and choosing the cursor shows the button on.
- **No "Set … from Here" items** in the photo's menu: picker cursors pick at the pointer instead.

### The API

Cursors are the app's. The engine, the session protocol, the CLI and MCP never hear of them: a cursor turns a finished gesture into commands its owner already has.

- **In `apertrust-tools`, data only:** `CursorDef { id, group, icon, key, order, gestures, surfaces, options }`, `CursorGroup` (Select, Pan, Pick, Mask, Retouch, Annotate, CopyApply, in the well's order), `CursorGroupDef { group, settings }`, `Gesture` and `Surface`; and `ToolDefinition::cursors: &'static [ToolCursor]`, `ToolCursor { cursor, picker }`, a tool's picker cursor naming one of its pickers, with an icon of its own so the Pick face shows which is active (`pick-white-balance`, `pick-exposure`), in Pick alone, as `menus` and `toolbar` are data ([0091](0091-context-menus-places-and-tools-items-as-actions.md), [0092](0092-the-toolbar.md), [0093](0093-one-tool-model-processing-tools-and-utility-tools.md)).
- **In the app:** `CursorInput` (a click, an area, a shape, a stroke, points, a photo, in the photo's coordinates with the keys held); one trait, `CursorOwner`, for every owner but a tool (the app's Select, Pan and Copy and apply; layers and masks; retouch; annotations): its cursors and group settings, whether a press can act now and why not, a gesture's live preview, its finished commands; one built-in owner for every tool's cursor; the mode, `Cursor { owner, id }`, Select at every start, never saved.
- **A picker cursor's click or drag is `PickerCommand::Once { picker, region, keys, settings, channel }`** ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)): a tool's picker on for one reading of that region, its pick applied, then off, one completed change, one step of source Auto that never merges where it turns the tool on or changes a value, and none where it changes nothing.

## Options considered

- **"Set … from Here" items in the photo's right-click menu,** a one-shot pick at the point clicked: no way to pick by a drag, the pick hidden behind a right-click no touch or keyboard user makes first, two clicks a pick for a series, and nothing for masks, retouch or copying edits.
- **Each feature its own mode in its own place** (pickers in their tools, masks in the Layers section, annotations in a bar of their own): several modes that can clash over one press, no shared keys, and each new feature inventing how its mode is chosen and shown.
- **Every cursor its own segment in the well,** none behind a list: the well would reach about 386 points with Phase 4's cursors and fold, some cursors moving into a menu at the end, so a cursor's place would change with the window's width.
- **The well on the viewer bar:** the viewer bar will carry darktable's view toggles beside Apertrust's own, the layer menu, the values and the zoom, and would overflow at ordinary widths.
- **A help line on the viewer bar** saying what the chosen cursor does: text on screen all the time for what a tooltip says once.
- **A plate behind the chosen cursor:** a filled shape among the Toolbar's plate-free icons; the accent marks what a press on the photo acts with, as amber marks what is chosen elsewhere.
- **Cursors replacing the picker button,** a press on it choosing its cursor: one concept, but darktable's live picker, which the migration mirrors, would go.
- **A cursor ending after one use,** as a menu item's pick does: a series would need the cursor chosen again for every photo.
- **The hand as the default:** a drag over a mark would pan where it should move the mark; Select moves a mark from a drag on it and pans from a drag on empty photo, as darktable's darkroom does.
- **Select never panning,** a drag on empty photo moving nothing: a departure from darktable's drag-to-pan, and the built pan gone from the default cursor.
- **Space for a held Pan:** Space is Next Photo; A is darktable's own key to force panning.
- **`Once` taking a point alone,** the session making the area: a drag could not pick the area drawn. A `Region` carries both, a point read as darktable's click reads it.
- **Cursors known to the engine:** a mode in the session for what is the interface's; every client already sends the commands a cursor sends.

## Consequences

- **[0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md):** `PickerCommand::Once` and `Region::pressed` (built, 79.17), sent by a picker cursor.
- **[0091](0091-context-menus-places-and-tools-items-as-actions.md):** its open question on an item that picks at the pointer is settled: no such item.
- **[0092](0092-the-toolbar.md):** the part `cursors`, one, the whole well, in the default list between two flexible spaces, never folded; a hidden well leaves every cursor with a key reachable by it, and every cursor's work by its tool's own button.
- **[0098](0098-actions-and-shortcuts-after-darktable.md):** the cursor actions' family and kind, and the default keys Esc, A held and K.
- **[0034](0034-layers-masked-tool-instances-in-a-fixed-order.md):** a mask's parts drawn from the Layers section's buttons or the Mask cursors, one state.
- **[0081](0081-icon-only-buttons-draw-no-plates-on-is-the-filled-icon.md):** a chosen cursor is shown by its icon in `accent_text`, not its filled form.
- **The built pan by a drag** stays Select's on empty photo, darktable's drag-to-pan; Pan's drag pans over marks too.
- **The code:** `apertrust-tools` gains the declarations and their checks, White balance's and Exposure's cursors first; `apertrust-ui` the well, in the gallery; the app the mode, the owners, the Viewer's gestures and pointer shapes, the actions and keys, the well on the Toolbar and the group settings in the settings schema; the engine `Once` alone. No new crate.
- **The docs:** [cursors.md](../cursors.md) holds the design and its open questions; pickers.md, gui.md, session-protocol.md, history.md, layers.md, crates.md and the roadmap follow it.
