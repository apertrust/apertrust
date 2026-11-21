# 0083. The Tools panel's layout, dragged and floating

## Context

[0038](0038-the-side-panels-tabs-by-editing-stage.md) gives the Default workspace its tabs as fixed lists: which sections each tab holds, in editing order, and Scopes pinned over the editing tabs, now `data/workspaces/default.toml` ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)). People want the few tools they use on every photo together, a section pinned where they need it, and a section such as Scopes in sight whatever tab is shown. How does a person change the Tools panel's layout, and how is it kept from ever changing the photo? Where it is kept, which tabs a workspace has and which tools it hides are [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s.

darktable at the reference release:

- **Utility modules** reorder by dragging their header, within a panel and to the other side panel, display only, the place kept per view as `plugins/<view>/<module>_position` (`src/libs/lib.c`, `_on_drag_motion` and `_on_drag_drop`).
- **Processing modules** reorder by dragging their header too, and that changes the pipeline order: `dt_ioppr_move_iop_before` or `_after`, a history item and a rebuilt pipe (`src/develop/imageop.c`, `_on_drag_motion`). Within a group the darkroom lists its modules in pipeline order; which modules a group holds is set in the module group presets.
- **No floating panels.**

Apertrust never reorders the processing (principles, What we will not build; [0022](0022-pipeline-order-and-the-default-pipeline.md), [0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)). Workspaces are interface state ([0037](0037-one-view-the-photos-tab.md)); every client reaches the engine through the session protocol ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)); eframe's viewports are rejected ([0041](0041-dialogs-as-native-modal-windows.md)).

## Decision

**Every section's place in the Tools panel is the workspace's layout, kept in the workspace's file, and a person changes it by dragging a section's header, through the header's ⋯ menu, with keys, or in the file.** The layout is display order only: it never reaches the engine, the edit or the history. Built: ⋯'s items and their keys, the routes a person has today, the screen readers' custom actions in the AccessKit tree, reaching screen readers once AccessKit's platform adapters pass custom actions on, and the workspace's file written (decision 96's step 4); planned: the drag in Phase 2, the floating panels in Phase 3 ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md), When).

### The layout model

- **A workspace is [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s:** its tabs, each tab's pinned and scrolling tools, the tools it hides, the tools that float and its toolbar, kept in `<user data>/workspaces/<id>.toml`, Default and Essentials built in, data files in `data/workspaces/` (`default.toml` and `essentials.toml`), Default's defaults [0038](0038-the-side-panels-tabs-by-editing-stage.md)'s lists. The file is where a person can always change a workspace, whatever an editor offers.
- **One model, one set of operations:** drag, ⋯, keys, the file and Phase 3's visual editor all change it through the same operations: move within a list, move to another tab at an index, pin at an index, unpin, hide, show in a tab, float, dock at a tab and index, reset a tab.
- **Floating** is the file's `[floating]`, keyed by section, of the tab it left. A floating section's id stays in its tabs' lists, so the place it left, pinned or not, survives a restart.
- **Kept app-wide** in `settings.toml`, as the Tools panel's width is, since they suit a screen rather than a task: a floating section's place and width, `[tools_panel.floating.<section>]`, with `corner` (`top-beside`, `top-away`, `bottom-beside` or `bottom-away`: the Viewer's corner it keeps its distance from, beside the Tools panel or away from it), `x` and `y`, that distance in points, and `width` in points. The schema gains a record kind, a table of named fields each of a kind, for it. A section's open or closed state is its one state in every workspace, docked or floating (`[edit.sections]`).
- **The defaults are [0038](0038-the-side-panels-tabs-by-editing-stage.md)'s lists:** the Default workspace's in `data/workspaces/default.toml`, embedded by the app and read by `crates/app/src/layout.rs` (`Layout::defaults`), plain data with no egui ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)); Essentials' in `data/workspaces/essentials.toml`, read the same way.
- **The layout shown,** a new build's new tool placed by its default tab or hidden, the invariants and where each part is kept are [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s.

### Display order only

- **Every section can be reordered on screen, processing tools included.** A tab starts in its default order, the Default workspace's the editing order ([0038](0038-the-side-panels-tabs-by-editing-stage.md)), and keeps it until the person moves something. This departs from darktable, whose drag of a processing module's header changes the pipeline order (`src/develop/imageop.c`); Apertrust's changes only what is shown. A utility section's drag is darktable's own (`src/libs/lib.c`).
- **Guaranteed three ways:** the layout lives in the app's `Shell` and the workspace's file, and no session command carries it; the engine never depends on the app; and `pipeline::plan` sorts instances by their manifest's slot whatever order it gets them in. Tested: a move, a float and a dock send no command on the edit, and the sidecar, history and render stay byte for byte the same. The one command a move can lead to is the Scopes request (`Command::Scopes`), which follows whether Scopes are shown and open, as it does after a tab is chosen.
- **So the departure is never misread:** a processing tool's drag says "Moves it in this tab only. Processing order stays fixed." under the lifted header; Ctrl+Shift with the drag, darktable's gesture, does what a plain drag does; the Tools panel's ⋮ has **Processing Order**, checked while the processing tools of each of the tab's two lists, the pinned and the scrolling, are in pipeline order, which sorts each list's processing tools among the places they hold in it: no tool crosses the pinned area's edge, and utilities keep their places, so a pinned tool stays pinned above the rest.
- **Layout moves are not edits:** no step in the edit's history ([0078](0078-undo-redo-and-the-edits-history.md)), never undone by Undo. The ways back are the opposite move, Processing Order, Reset Tab, Workspace ▸'s Restore Defaults, Dock All Floating Panels. Reset Tab puts the tab back as it shipped: its two lists become its defaults again, a section it brings back leaves `hidden` and any other tab whose defaults lack it, so a move out of the tab is undone while a section shipped in several tabs, Scopes, stays in each; a section the reset takes out of the tab and no other tab holds goes to its default tabs the workspace has, else it is hidden; a floating one is never hidden, but stays floating and docks later at its default place (below). A tab of a person's own, with no defaults, has no Reset Tab. Workspace ▸'s Restore Defaults deletes a built-in workspace's file, so the one shipped applies again; a person's own has none ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)).

### Dragging a header

- **The handle** is the header but its buttons: the title, the chevron and the gap before the buttons, the sticky header (`PanelBody::header`) too. It senses click and drag (`Section::draggable`; `apertrust-ui` never learns what the key means). A press that moves 6 points, egui's click distance, is a drag; one released before is a click that opens or closes the section, as now. A press held 0.8 s and released without moving does neither, egui's rule kept.
- **While dragging:** the lifted header follows the pointer, no wider than 320 points; the source stays in place, its title dimmed, so nothing reflows; the insertion line, 2 points in the accent, shows the gap nearest the pointer; one line of text under the lifted header says what a release does. The list scrolls within 32 points of its edges.
- **The drop targets:**

  | Over | A release |
  | --- | --- |
  | A gap in the scrolling list | Moves it there, unpinned |
  | A gap in the pinned area | Moves it there and pins it in this tab |
  | Another tab's icon | Moves it to that tab, which stays unshown; held 0.6 s, the tab opens under the drag |
  | The Hidden tab's icon ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)) | Hides it, as ⋯ › Remove Tool does; Export takes no such drop |
  | Its own place, the current tab, a tab already holding it, a disabled tab, », ⋮, the Hidden tab's list | Nothing: the icon takes no drop and opens nothing under the drag; nothing in the Hidden tab is reordered or pinned |
  | The Viewer | Floats it there |

- **Cancel:** Esc, a right-click, the window losing focus, or a dialog opening; nothing changes. egui's `DragAndDrop` takes Esc first and clears the payload; the app clears it on losing focus.
- **The drop** is applied after the frame, as every interface intent is, and the workspace's file is written once the layout has stood still for a second, as after every layout change ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)); nothing is written during the drag. The moved header keeps the focus where it stays in the tab shown; moved to another tab, the focus goes to that tab's icon.
- **Each section is drawn in a `Ui` with a stable id,** `UiBuilder::scope_id(("section", id))`, so a value field's draft, a slider's drag or a group's animation keep their state when the section moves to another list, tab or a floating panel.

### Pinning, the ⋯ menu, keys and screen readers

- **Pinning by drag:** dropping a section in the pinned area pins it in that tab; dragging it out unpins it. Pinned state is per tab. There is still no pin button.
- **The section's ⋯ menu** holds its layout items, a tool's and a utility's alike, through the popup service ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)): Pin Tool or Unpin Tool (whichever applies), Move Tool Up and Move Tool Down (one place in its list, disabled at its ends), Move Tool to Tab › (each tab; the one it is in and a disabled tab disabled), Remove Tool, which hides it, its edit staying, and for a hidden tool Show Tool in Tab › alone in their place ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)); Float or Dock; on a floating panel, Position › with its four corners. They come last, after the app's editing items and the tool's own ([0091](0091-context-menus-places-and-tools-items-as-actions.md)).
- **Every move has a menu route:** ⋯'s Move Tool Up and Move Tool Down are the single-pointer alternative to the drag that WCAG 2.2's 2.5.7 asks for when reordering within a tab, as Move Tool to Tab ›, Pin Tool and Remove Tool are for moving between tabs, pinning and hiding ([0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)).
- **A section's header takes no right-click,** a tool's or a utility's, nor Shift+F10 or the Menu key ([0091](0091-context-menus-places-and-tools-items-as-actions.md)): a tool's presets come from its header's presets button, and ⋯ opens from its own button.
- **Keys:** on a focused header Alt+↑ and Alt+↓ (Option on a Mac) move it one place in its list, part of the header's own keys; on a floating panel's focused header Alt+Shift+arrows move it 10 points. Registry actions with no default keys, bindable, their words [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s: `section-move-up` and `section-move-down` (Move Tool Up and Move Tool Down), `section-pin`, `section-move-to-tab-1` to `-12`, `section-hide` (Remove Tool), `section-show`, `section-float` and `section-dock`, each acting on the section whose header has the focus; `dock-all`, available while a section floats, and `tab-processing-order`, acting on the tab shown, need no header's focus.
- **Screen readers:** the header keeps its Button role and gains AccessKit custom actions Move Tool Up, Move Tool Down, Pin Tool or Unpin Tool, Remove Tool or Show Tool in Tab, Float and Dock; after any move a polite live region says where it went ("Exposure moved to 3 of 12 in Adjust", "Exposure pinned in Adjust", "Vignetting hidden", "Exposure floating", "Exposure docked in Adjust"), in every language's messages.

### Another tab

- **A drop on a tab's icon** puts a processing tool at its pipeline place among the tab's processing tools, by its nearest neighbour there; a utility at the top of the scrolling list; unpinned. The spring-loaded tab places it exactly. A hidden tool dragged from the Hidden tab or a search's list lands the same way and leaves `hidden`.
- **A move never copies:** the section leaves the tab it was dragged from and lands in another, so a tool whose only tab that was can move as any section can. A section in several tabs, such as Scopes, moves out of the tab it was dragged from only.
- **Any section in any tab:** the Tools panel draws every tab's sections by id through one dispatcher (a tool, Collections, Scopes, History, later the rest). `reveal`, View › Collections' "first tab holding Collections" and the Scopes counts follow the layout, not the constants: Scopes are counted whenever they are shown and open, in a tab or floating.

### Floating

- **A floating section is a panel inside the app's window,** over the Viewer, never an OS window: an `egui::Area` in the main window's context, below the viewer bar, never over the Tools panel, the Toolbar or the Browser, under popups, tooltips and dialogs. It is the same section, drawn by the same function, sending the same `Change`s, solid with the popup shadow and a border. Out of the Tools panel by a drag over the Viewer, or ⋯ › Float, which places it near the Tools panel's top.
- **It belongs to no tab.** It stays visible whatever tab is shown, and no tab lists anything for it while it floats. The layout remembers the place it left: its id stays in its tabs' lists, hidden while it floats, and `floating` names the tab it left.
- **Docking:** dropped into any tab, at a gap or on a tab's icon, it docks there, which is also a way to move it to another tab (it leaves the tab it left, other tabs keeping it). Into a tab that already holds it, it docks at the gap, its place there moved to it, and the tab it left keeps it. Its ×, named for the tab it left ("Return to Adjust"), ⋯ › Dock and Dock All Floating Panels return it to the place it left, or, where that tab no longer holds it (after Reset Tab, say, or with the tab gone), to its default place: its default tab, placed there as a new section is, or, with none of its default tabs left in the workspace, the end of the scrolling list of the workspace's first tab that holds tools. Reset Tab never docks a floating section, and a floating section cannot be hidden until it docks.
- **Its width** starts as the Tools panel's and is changed at its side edges, 240 to 600 points, with the edge's keys; its height is its content's, up to the Viewer's less 16 points, scrolling beyond. It keeps its distance from its corner as the window, the Tools panel or the Browser change size, pushed inside where an edge would cut it; where the Viewer is too small for it, it shows docked at the place it left until there is room, nothing written. A press on it brings it to the front.
- **Keys and focus:** F6 visits floating panels after the Tools panel, a region named Floating panels, each to AccessKit a pane named by its section; the view's keys work as everywhere, H peeking at it too. The photo takes no click under it.
- **With no photo open,** each section follows its own rule wherever it sits, docked in any tab or floating: a tool's and Scopes' controls are drawn disabled, while Collections and the Photos tab's other sections work as ever. Every tab stays choosable.
- **Which sections float** is the workspace's; each one's place and width are app-wide, so it comes back where it was.

### When

[0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s steps, each one pull request reviewed before the next: in Phase 2 the layout model with its workspaces (step 1), ⋯'s layout items with their keys and screen readers' actions (step 4), Reset Tab and Processing Order in the tab bar's menu and ⋮ (step 5), and dragging a header, within a tab, into the pinned area and to another tab through its icon (step 7); in Phase 3 the floating panels inside the window, with Float, Dock, Position, Dock All Floating Panels and their keys and actions. The Customise tabs dialog is not built on fixed lists: Phase 3's visual editor for workspaces takes its place.

## Options considered

- **Reordering utility sections only, as darktable's libs:** tools would keep pipeline order within a tab, and could only move to another tab or be pinned. The list would keep reading as the pipe, but the tools used together could not sit together, which is the point; Processing Order sorts a tab into the pipe's order instead.
- **No reordering:** tabs and order changed only in an editor, as darktable's group presets are. A dialog for what one drag does, and nothing at all until the dialog is built.
- **Floating panels as OS windows,** through the dialog host for a second monitor: a surface, key routing and a kind of window per platform for each, no placement on Wayland, and no dragging back, since moving an OS window sends the app no pointer events; eframe's viewports give a child window no AccessKit in eframe 0.37 ([0041](0041-dialogs-as-native-modal-windows.md)). Not built.
- **A placeholder in the tab while a section floats,** a header line showing and docking it: a row in a tab for a panel visible in every tab; the layout keeping the place it left does that job.
- **A second, live copy in the tab while it floats:** two copies of one tool, against one tool for each task (principles), and a section drawn twice a frame.
- **Floating positions per workspace:** a position suits a screen, not a task, as the Tools panel's width does.
- **Only Pin and Unpin in ⋯,** with the rest by dragging: [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md) weighs it.
- **Pinning only in an editor:** the pinned area is already a list the drag can reach, and the editor comes later.
- **The moves stored as a diff replayed on the defaults,** or a number per section as darktable's `_position`: the first breaks as defaults change, the second needs renumbering whenever a tool is added.
- **The layout in `settings.toml`, under `[workspaces.<id>]`:** [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md) weighs it against a file per workspace.
- **A second dock column on the other side:** a different feature, darktable's two side panels; it narrows the photo.

## Consequences

- [0038](0038-the-side-panels-tabs-by-editing-stage.md)'s lists are the Default workspace's defaults and [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s Essentials' lists its; the tests holding each to its record stay on the defaults.
- `crates/app`: `layout.rs` (the model, the workspace's file, resolving, the operations), the tabs drawn by section id through one dispatcher, `reveal`, `revealing`, Collections' tab and `scopes_wanted` from the layout, the floating areas, the actions and their messages in four languages.
- `apertrust-ui`: `Section::draggable`, `PanelBody`'s drop targets, insertion line and auto-scroll, the tab bar's drop targets, the floating panel's frame; snapshots and kittest drags for each.
- `data/settings.schema.toml`: the record kind and `tools_panel.floating`; the workspace's file is [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)'s.
- The `ui-conventions` skill: a tool's header has ⋯ with the layout items, pinning by drop, the draggable header; `desktop-architecture`: sections drawn under stable scope ids, one place a frame.
- Decisions [0018](0018-one-submit-gate-for-the-engine-and-the-window.md), [0022](0022-pipeline-order-and-the-default-pipeline.md), [0032](0032-tools-off-until-first-touched.md), [0034](0034-layers-masked-tool-instances-in-a-fixed-order.md) and [0041](0041-dialogs-as-native-modal-windows.md) are unchanged: no new surface, no new window, the order fixed.
