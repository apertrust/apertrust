# 0083. The side panel's layout, dragged and floating

## Context

[0038](0038-side-panel-tabs-as-darktables-groups.md) gives the Edit workspace darktable's groups as fixed lists: which sections each tab holds, in pipeline order with the last module at the top, and Scopes pinned over the four groups (`TAB_MODULES` and `SCOPES_PINNED` in `crates/app/src/main.rs`). People want the few modules they use on every photo together, a section pinned where they need it, and a section such as Scopes in sight whatever tab is shown. How does a person change the side panel's layout, where is it kept, and how is it kept from ever changing the photo?

darktable at the reference release:

- **Utility modules** reorder by dragging their header, within a panel and to the other side panel, display only, the place kept per view as `plugins/<view>/<module>_position` (`src/libs/lib.c`, `_on_drag_motion` and `_on_drag_drop`).
- **Processing modules** reorder by dragging their header too, and that changes the pipeline order: `dt_ioppr_move_iop_before` or `_after`, a history item and a rebuilt pipe (`src/develop/imageop.c`, `_on_drag_motion`). Within a group the darkroom lists its modules in pipeline order; which modules a group holds is set in the module group presets.
- **No floating panels.**

Apertrust never reorders the processing (principles, What we will not build; [0022](0022-pipeline-order-and-the-default-pipeline.md), [0034](0034-layers-masked-module-instances-in-a-fixed-order.md)). Workspaces are interface state ([0037](0037-one-view-the-photos-tab.md)); every client reaches the engine through the session protocol ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)); eframe's viewports are rejected ([0041](0041-dialogs-as-native-modal-windows.md)).

## Decision

**Every section's place in the side panel is the workspace's layout, kept in settings, and a person changes it by dragging a section's header, through the header's ⋯ menu, with keys, or in the settings file.** The layout is display order only: it never reaches the engine, the edit or the history. Planned: built in Phase 3 with named workspaces, the floating panels in the UX pass that follows it.

### The layout model

- **The app defines its built-in workspaces, and the settings file is where a person can always change them**, whatever a dialog offers. The layout model is the settings' shape; drag, ⋯, keys, the file and later Customise tabs all change the same model through the same operations: move within a list, move to another tab at an index, pin at an index, unpin, float, dock at a tab and index, reset a tab.
- **With named workspaces,** each workspace's layout is this model, Edit's tabs [0038](0038-side-panel-tabs-as-darktables-groups.md)'s. What the gestures and the file change is each tab's sections, their order, which are pinned, and which float. Adding, naming or removing tabs and leaving sections out of a workspace are Customise tabs', on the same model.
- **Customise tabs** is built in Phase 2 as [gui.md](../gui.md#build-order)'s step 7 plans it, editing the Edit workspace's fixed tab lists, and is reworked onto the layout model in Phase 3, editing it through the same operations.
- **Kept per workspace,** under `[workspaces.<id>]`:

  | Key | Holds | Written |
  | --- | --- | --- |
  | `tab` | The tab last shown | When a tab is chosen |
  | `pinned.<tab>` | The tab's pinned sections, top first | With `order.<tab>`, once either changes |
  | `order.<tab>` | The tab's other sections, top first | With `pinned.<tab>`, once either changes |
  | `floating.<section>` | That the section floats, and the tab it left | When it floats; removed when it docks |
  | `hidden` | The sections the workspace leaves out, never a module in Edit | By Customise tabs, from Phase 3 |

  ```toml
  [workspaces.edit.pinned]
  base = ["scopes"]

  [workspaces.edit.order]
  base = ["exposure", "agx", "tone_curve", "toneequal", "flip", "demosaic", "highlights", "temperature", "rawprepare"]

  [workspaces.edit.floating]
  scopes = "base"
  ```

  `pinned` and `order` are tables keyed by a new `tabs` registry, each a list of section ids; `floating` a table keyed by `sections`, of a tab id; `hidden` a list of section ids. A floating section's id stays in its tabs' lists, so the place it left, pinned or not, survives a restart.
- **Kept app-wide,** as the side panel's width is, since they suit a screen rather than a task: a floating section's place and width, `[side_panel.floating.<section>]`, with `corner` (`top-beside`, `top-away`, `bottom-beside` or `bottom-away`: the central panel's corner it keeps its distance from, beside the side panel or away from it), `x` and `y`, that distance in points, and `width` in points. The schema gains a record kind, a table of named fields each of a kind, for it. A section's open or closed state is its one state in every workspace, docked or floating (`side_panel.sections`).
- **The defaults are [0038](0038-side-panel-tabs-as-darktables-groups.md)'s lists:** `TAB_MODULES` and `SCOPES_PINNED` become the Edit workspace's defaults, `Layout::defaults()` in `crates/app/src/layout.rs`, plain data with no egui.
- **The layout shown** is worked out when settings are read and after each change, never per frame:
  1. each tab takes its kept lists if it has them, else its defaults;
  2. an id this build does not know is skipped and left in the file, as unknown settings are;
  3. a section this build has that no tab of the workspace holds, and that `hidden` does not name, is new: it goes into its default tab, right after the nearest section that comes before it in the default list and is in that tab now, or first if none is. A tab nobody changed shows each new build's defaults; a moved module is not new.
- **Invariants,** held by the operations and unit-tested: a section at most once in a tab, `pinned` and `order` never sharing an id; every module in at least one tab of Edit; a section drawn in one place a frame.
- **Version** stays 1: the keys are new, and an older build keeps them as keys it does not know and shows the defaults.

### Display order only

- **Every section can be reordered on screen, processing modules included.** A tab starts in darktable's order, the last module at the top, and keeps it until the person moves something. This departs from darktable, whose drag of a processing module's header changes the pipeline order (`src/develop/imageop.c`); Apertrust's changes only what is shown. A utility section's drag is darktable's own (`src/libs/lib.c`).
- **Guaranteed three ways:** the layout lives in the app's `Shell` and its settings, and no session command carries it; the engine never depends on the app; and `pipeline::plan` sorts instances by their manifest's slot whatever order it gets them in. Tested: a move, a float and a dock send no command on the edit, and the sidecar, history and render stay byte for byte the same. The one command a move can lead to is the Scopes request (`Command::Scopes`), which follows whether Scopes are shown and open, as it does after a tab is chosen.
- **So the departure is never misread:** a processing module's drag says "Moves it in this tab only. Processing order stays fixed." under the lifted header; Ctrl+Shift with the drag, darktable's gesture, does what a plain drag does; the side panel's ⋮ has **Processing Order**, checked while the processing modules of each of the tab's two lists, the pinned and the scrolling, are in pipeline order, which sorts each list's processing modules among the places they hold in it: no module crosses the pinned area's edge, and utilities keep their places, so a pinned module stays pinned above the rest.
- **Layout moves are not edits:** no step in the edit's history ([0078](0078-undo-redo-and-the-edits-history.md)), never undone by Undo. The ways back are the opposite move, Processing Order, the ⋮'s Reset Tab, Dock All Floating Panels, and from Phase 3 Reset Workspace. Reset Tab puts the tab back as it shipped: its two lists are removed from settings, so its defaults apply again, and a section it brings back leaves any other tab whose defaults lack it, so a move out of the tab is undone while a section shipped in several tabs, Scopes, stays in each; a section the reset takes out of the tab and no other tab holds goes to its default place, as a new section does.

### Dragging a header

- **The handle** is the header but its buttons: the title, the chevron and the gap before the buttons, the sticky header (`PanelBody::header`) too. It senses click and drag (`Section::draggable`; `apertrust-ui` never learns what the key means). A press that moves 6 points, egui's click distance, is a drag; one released before is a click that opens or closes the section, as now. A press held 0.8 s and released without moving does neither, egui's rule kept.
- **While dragging:** the lifted header follows the pointer, no wider than 320 points; the source stays in place, its title dimmed, so nothing reflows; the insertion line, 2 points in the accent, shows the gap nearest the pointer; one line of text under the lifted header says what a release does. The list scrolls within 32 points of its edges.
- **The drop targets:**

  | Over | A release |
  | --- | --- |
  | A gap in the scrolling list | Moves it there, unpinned |
  | A gap in the pinned area | Moves it there and pins it in this tab |
  | Another tab's icon | Moves it to that tab, which stays unshown; held 0.6 s, the tab opens under the drag |
  | Its own place, the current tab, a disabled tab, », ⋮ | Nothing |
  | The central panel | Floats it there |

- **Cancel:** Esc, a right-click, the window losing focus, or a dialog opening; nothing changes. egui's `DragAndDrop` takes Esc first and clears the payload; the app clears it on losing focus.
- **The drop** is applied after the frame, as every interface intent is, and written to settings at once; nothing is written during the drag. The moved header keeps the focus.
- **Each section is drawn in a `Ui` with a stable id,** `UiBuilder::scope_id(("section", id))`, so a value field's draft, a slider's drag or a group's animation keep their state when the section moves to another list, tab or a floating panel.

### Pinning, the ⋯ menu, keys and screen readers

- **Pinning by drag:** dropping a section in the pinned area pins it in that tab; dragging it out unpins it. Pinned state is per tab. There is still no pin button.
- **The section's ⋯ menu** holds its layout items, a module's and a utility's alike, through the popup service ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)): Move Up, Move Down, Move to Pinned Area or Move to Scrolling Area (whichever applies), Move to Tab › (each tab; the one it is in and a disabled tab disabled), Float or Dock; on a floating panel, Position › with its four corners. A module's ⋯, drawn disabled until now, is enabled with them.
- **A right-click on a module's header** opens its presets, as darktable's does (`imageop.c`); on a utility's header it opens ⋯. Shift+F10 and the Menu key on a focused header open what a right-click opens.
- **Keys:** on a focused header Alt+↑ and Alt+↓ (Option on a Mac) move it one place in its list, part of the header's own keys; on a floating panel's focused header Alt+Shift+arrows move it 10 points. Registry actions with no default keys, bindable: `section-move-up`, `section-move-down`, `section-pin`, `section-move-to-tab-1` to `-6`, `section-float` and `section-dock`, each acting on the section whose header has the focus; `dock-all`, available while a section floats, and `tab-processing-order`, acting on the tab shown, need no header's focus.
- **Screen readers:** the header keeps its Button role and gains AccessKit custom actions Move up, Move down, Pin, Float and Dock; after any move a polite live region says where it went ("Exposure moved to 3 of 9 in Base", "Exposure pinned in Base", "Exposure floating", "Exposure docked in Base"), in every language's messages.

### Another tab

- **A drop on a tab's icon** puts a processing module at its pipeline place among the tab's processing modules, by its nearest neighbour there; a utility at the top of the scrolling list; unpinned. The spring-loaded tab places it exactly.
- **A move never copies:** the section leaves the tab it was dragged from and lands in another, so a module whose only tab that was can move as any section can, and every module stays in a tab of Edit. A section in several tabs, such as Scopes, moves out of the tab it was dragged from only.
- **Any section in any tab:** the side panel draws every tab's sections by id through one dispatcher (a module, Browse, Scopes, History, later the rest). `reveal`, View › Browse's "first tab holding Browse" and the Scopes counts follow the layout, not the constants: Scopes are counted whenever they are shown and open, in a tab or floating.

### Floating

- **A floating section is a panel inside the app's window,** over the central panel, never an OS window: an `egui::Area` in the main window's context, below the central bar, never over the side, top or grid panel, under popups, tooltips and dialogs. It is the same section, drawn by the same function, sending the same `Change`s, solid with the popup shadow and a border. Out of the side panel by a drag over the central panel, or ⋯ › Float, which places it near the side panel's top.
- **It belongs to no tab.** It stays visible whatever tab is shown, and no tab lists anything for it while it floats. The layout remembers the place it left: its id stays in its tabs' lists, hidden while it floats, and `floating` names the tab it left.
- **Docking:** dropped into any tab, at a gap or on a tab's icon, it docks there, which is also a way to move it to another tab (it leaves the tab it left, other tabs keeping it). Its ×, named for the tab it left ("Return to Base"), ⋯ › Dock and Dock All Floating Panels return it to the place it left, or, where that tab no longer holds it (after Reset Tab, say, or with the tab gone), to its default place: its default tab, after its nearest default neighbour there, as a new section goes. Reset Tab never docks a floating section.
- **Its width** starts as the side panel's and is changed at its side edges, 240 to 600 points, with the edge's keys; its height is its content's, up to the central panel's less 16 points, scrolling beyond. It keeps its distance from its corner as the window, the side panel or the grid panel change size, pushed inside where an edge would cut it; where the central panel is too small for it, it shows docked at the place it left until there is room, nothing written. A press on it brings it to the front.
- **Keys and focus:** F6 visits floating panels after the side panel, a region named Floating panels, each to AccessKit a pane named by its section; the view's keys work as everywhere, H peeking at it too. The photo takes no click under it.
- **With no photo open,** each section follows its own rule wherever it sits, docked in any tab or floating: a module's and Scopes' controls are drawn disabled, while Browse and the Photos tab's other sections work as ever. Every tab stays choosable.
- **Which sections float** is the workspace's; each one's place and width are app-wide, so it comes back where it was.

### When

Steps 0 to 4 in Phase 3, with named workspaces, and step 5, the floating panels, in the UX pass after Phase 3; in this order, each step one pull request reviewed before the next:

| Step | Delivers |
| --- | --- |
| 0 | The Edit workspace's settings keys as [gui.md](../gui.md#what-the-file-looks-like) has them: `[workspaces.edit] tab`, `[side_panel.sections]`, `[side_panel.groups]`, the old `[edit]` read into them |
| 1 | The layout model and its settings, with Processing Order and Reset Tab; the four group tabs drawn from the layout shown |
| 2 | Moving by drag within a tab, pinning by drag included |
| 3 | The ⋯ menu's items, the keys and the screen readers' actions and announcements, but for floating's |
| 4 | Another tab: drops on a tab, the spring-loaded tab, Move to Tab ›; any section in any tab |
| 5 | Floating panels inside the window, with Float, Dock, Position, Dock All Floating Panels and their keys and actions |

Customise tabs moves onto the layout model once step 1 is built.

## Options considered

- **Reordering utility sections only, as darktable's libs:** modules would keep pipeline order within a tab, and could only move to another tab or be pinned. The list would keep reading as the pipe, but the modules used together could not sit together, which is the point; darktable's order stays the default instead, and Processing Order brings it back.
- **No reordering:** tabs and order changed only in Customise tabs, as darktable's group presets are. A dialog for what one drag does, and nothing at all until the dialog is built.
- **Floating panels as OS windows,** through the dialog host for a second monitor: a surface, key routing and a kind of window per platform for each, no placement on Wayland, and no dragging back, since moving an OS window sends the app no pointer events; eframe's viewports give a child window no AccessKit in eframe 0.37 ([0041](0041-dialogs-as-native-modal-windows.md)). Not built.
- **A placeholder in the tab while a section floats,** a header line showing and docking it: a row in a tab for a panel visible in every tab; the layout keeping the place it left does that job.
- **A second, live copy in the tab while it floats:** two copies of one module, against one module per tool (principles), and a section drawn twice a frame.
- **Floating positions per workspace:** a position suits a screen, not a task, as the side panel's width does.
- **Pinning only in a dialog** (Customise tabs): the pinned area is already a list the drag can reach, and the dialog is not built.
- **The moves stored as a diff replayed on the defaults,** or a number per section as darktable's `_position`: the first breaks as defaults change, the second needs renumbering whenever a module is added.
- **A second dock column on the other side:** a different feature, darktable's two side panels; it narrows the photo.

## Consequences

- [0038](0038-side-panel-tabs-as-darktables-groups.md)'s lists are the Edit workspace's defaults; the test holding them to `OPERATIONS` reversed stays on the defaults.
- `crates/app`: `layout.rs` (the model, resolving, the operations), the tabs drawn by section id through one dispatcher, `reveal`, `revealing`, Browse's tab and `scopes_wanted` from the layout, the floating areas, the actions and their messages in four languages.
- `apertrust-ui`: `Section::draggable`, `PanelBody`'s drop targets, insertion line and auto-scroll, the tab bar's drop targets, the floating panel's frame; snapshots and kittest drags for each.
- Customise tabs, built in Phase 2 on the fixed lists, is reworked in Phase 3 to edit the layout model.
- `data/settings.schema.toml`: the `tabs` registry, the record kind, `workspaces.edit.pinned`, `order` and `floating`, a workspace's `hidden`, and `side_panel.floating`.
- The `ui-conventions` skill: a module's header has ⋯ with the layout items, pinning by drop, the draggable header; `desktop-architecture`: sections drawn under stable scope ids, one place a frame.
- Decisions [0018](0018-one-submit-gate-for-the-engine-and-the-window.md), [0022](0022-pipeline-order-and-the-default-pipeline.md), [0032](0032-modules-off-until-first-touched.md), [0034](0034-layers-masked-module-instances-in-a-fixed-order.md) and [0041](0041-dialogs-as-native-modal-windows.md) are unchanged: no new surface, no new window, the order fixed.
