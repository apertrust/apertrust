# 0092. The Toolbar

## Context

The Toolbar is each main window's title bar ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)). It is drawn from the workspace's list of item ids, by default the menu button on Windows and Linux, Import…, Export…, Reset, Undo, Redo, Copy edit and Apply edit, the cursors' well, the jobs spinner and the save state. As it narrows it folds the actions into ⋯ More actions.

People want the actions they use on every photo within reach, and tools want buttons of their own there, such as Orientation's rotations. Several things constrain how:

- The window must stay movable by the empty parts of the bar.
- The caption buttons and the Mac's traffic lights are the platform's.
- Every command is a registry action ([0036](0036-the-design-system.md)).
- Tools add items as data and run no code in the interface ([0091](0091-context-menus-places-and-tools-items-as-actions.md)).
- What a person arranges belongs to the workspace, with the workspace's file always a way to change it ([0083](0083-the-side-panels-layout-dragged-and-floating.md), [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)).

## Decision

**The Toolbar is one a person customises.** It is an ordered list of item ids, kept for each workspace and drawn by the app from the registry. Everything but the caption buttons and the Mac's traffic lights may be moved or hidden. Built: tools' items, the set of what can be placed, and the list, kept in each workspace's file and drawn from that set, with the guards but Show Menu Button. Planned: Phase 3 builds its menu, the overflow and labels; the Customise Toolbar window comes much later.

### The list and its defaults

There is one ordered list of item ids for each workspace. The caption buttons and the traffic lights are not in it: they stay where the platform puts them, and the toolbar fills the room between its start inset and its end.

| Platform | The default list, today's order |
| --- | --- |
| macOS | (the traffic lights) `import`, `export`, `cull` (Phase 3), `reset`, `undo`, `redo`, `space`, `copy-edit`, `apply-edit`, `flexible-space`, `cursors`, `flexible-space`, `jobs`, `assistant` (Phase 6), `save-state` |
| Windows, Linux | `menu-button`, then the same, then 16 points (`spacing.xl`) before minimise, maximise and close |

- There is one list for every platform. The Mac never shows `menu-button` and never takes it out of the list.
- Before and After, Clipping Warnings and Zoom to 100% are offered but not in the default, since the viewer bar has them. So is Settings….
- **The cursors' well,** `cursors`, stands between two flexible spaces, centred in the room between the actions and the status slots ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md), [cursors.md](../cursors.md#the-well)). Hidden, it leaves every cursor with a key reachable by it, and every cursor's work by its tool's own button, so a cursor with no key, as Exposure's, waits for the well or a key the person gives it: Select on Esc, Pan on A held, a picker's work by its tool's picker button, a mask's by the Layers section's buttons, Copy and apply's by Copy edit and Apply edit.

**What can be placed:**

| Kind | Ids | How many |
| --- | --- | --- |
| Parts | `menu-button` (Windows and Linux, drawn as the app's icon or ☰ as the setting Menu button says, [0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)), `jobs`, `save-state`, `assistant` (Phase 6), and `cursors`, the whole well of cursors, every group in it ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)) | One each |
| The app's actions | Only actions that belong to no tool, pressed or toggled, to which the registry gives an icon: Open…, Open Folder…, Settings…, Full Screen, Zoom to 100%, Zoom to Fit, Jobs (`show-jobs`), Next Photo and Previous Photo; Cull, Before and After, Clipping Warnings and Show in Folder as their actions are built. Import… is here until the Import tool, which then declares it | One each |
| Tools' actions | A tool's own items, which it declares (Tools' items, below): a processing tool's `tool-<tool>-<action>`, generated from its own actions as decision 91 built them, or a utility's registry action, as History's `undo`; the item's id is the action's | One each |
| Spacing | `space`, 16 points; `flexible-space`, sharing what is left | Any number; the overflow removes fixed spaces as it narrows |

- **Never placeable:** hold actions (H), focus moves (F6), the members of a set (the backgrounds, the workspaces), and the platform's own items.
- **No separator line:** groups are told apart by space alone.
- **Parts' ids are their own.** A test fails if a part's id is a registry action's id, as `menu-button` is not the `menu` action.

**Tools' items.** An action that belongs to a tool is offered for the toolbar by that tool, in its definition, so a tool's buttons travel with it, a third-party tool's included; only actions that belong to no tool are the app's.

- A tool offers actions with `ToolDefinition::toolbar: &'static [ToolbarItem]`, in the parent every tool's definition embeds, a processing tool's and a utility's alike ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)), where `ToolbarItem { action, icon }`:
  - `action`, for a processing tool, names one of its own actions as its menu items do, `rotate_ccw`, which the registry holds as `tool-<tool>-<action>`; for a utility, the registry action it names, as History's `reset` in its menus;
  - `icon` names one of `apertrust-ui`'s icons, `export`, or is none for a `Row::Buttons` action, which takes its button's icon.
- The definition's check refuses an empty action, an action a processing tool does not have, the same action twice, no icon on an action that is no `Row::Buttons` button's, a utility's always, and more than two items on a processing tool; a utility has no limit. App tests fail on a declared action the registry does not hold and on an icon name `apertrust-ui` does not have.
- Offering an action only makes it a candidate: it is on the toolbar only where a person places it.
- The app lists every candidate in one stable order, the parts, the app's own actions, every tool's items in the tools' order, then `space` and `flexible-space`, each with its id, its kind, its icon and its words' id: the Toolbar draws from that list and the Customise Toolbar window lists it.

The items declared:

| Item | Id | Declared by | Icon |
| --- | --- | --- | --- |
| Export… | `export` | Export | `export` |
| Export Again | `export-again` | Export | `export` |
| Reset | `reset` | History | `reset` |
| Undo | `undo` | History | `undo` |
| Redo | `redo` | History | `redo` |
| Copy edit | `copy-edit` | History | `copy` |
| Apply edit | `apply-edit` | History | `apply` |
| Copy or Apply Parts… | `copy-apply-parts` | History | `copy` |
| Collections | `show-collections` | Collections | `library` |
| Rotate 90° anticlockwise | `tool-flip-rotate_ccw` | Orientation | its button's, `rotate-left` |
| Rotate 90° clockwise | `tool-flip-rotate_cw` | Orientation | its button's, `rotate-right` |

The app's own, with the icons the registry gives them: `import` (`import`, until the Import tool), `open` (`photo`), `open-folder` (`folder-open`), `settings` (`settings`), `full-screen` (`maximise`), `zoom-100` (`zoom-in`), `zoom-fit` (`zoom-out`), `show-jobs` (`process`), `next-photo` (`chevron`) and `previous-photo` (`back`).

**Guards.** A status part can be hidden only in its quiet states, and what it guards still works:

| Hidden | What still holds |
| --- | --- |
| The menu button | F10, Alt alone on Windows and Alt with an access key still open the menus, hung from the toolbar's start; View › Panels and the toolbar menu gain Show Menu Button, so the keys restore it |
| The save state | A failed write still shows its warning and Not saved at the toolbar's end; the message over the photo, the live region and the quit alert are unchanged |
| The jobs | The spinner shows while a failed job is left; View › Jobs and Ctrl+Shift+J still work |
| The assistant | It shows whenever an assistant is connected |
| The cursors | Every cursor keeps its action and key, and its work its tool's own button; the cursor chosen stays chosen |
| Everything | The toolbar is all empty area and still moves the window |

**Moving the window.** These are empty areas: they move the window and take a double-click, as the Toolbar's empty areas do (39):

- flexible spaces;
- the parts of a fixed space outside egui's 5-point margin round a control;
- a status slot showing nothing.

The overflow always keeps 24 points of them. On Windows the hit map's `HTCAPTION` rectangles come from the toolbar's layout, the items are `HTCLIENT`, and `HTMAXBUTTON` stays over the maximise button, so the snap layouts flyout is unchanged.

### The toolbar menu

**A right-click anywhere on the toolbar opens the toolbar menu,** on an item or on empty space, on every platform, as does Shift+F10 or the Menu key on a focused item. It is an app-only place of decision 91's, `Place::Toolbar`. It holds:

1. Hide "Undo", on an item;
2. Show Menu Button, while the menu button is hidden;
3. a separator;
4. Toolbar Labels, checked while on;
5. Customise Toolbar…, drawn disabled until its window is built, its tip "Customise Toolbar comes in a later version".

**Every item is reachable from the keys,** as decision 91 asks, whatever the toolbar shows: Hide "Undo" from Shift+F10 or the Menu key on that item, which is focusable while it is shown; the others in View › Panels, which F10 opens while the menu button is hidden. Show Menu Button is there on Windows and Linux only, the Mac never showing the menu button. A toolbar with nothing focusable needs no keyboard route of its own, since nothing in its menu acts on it alone.

**The system's window menu** is no longer on a right-click on the toolbar on Windows and Linux. It stays on the system's own key, Alt+Space on Windows and GNOME; X11 still has none, and the Mac's conventions are unchanged.

### Customising

- **The workspace's file** is the full way to customise until a window exists, and stays one after it (83, [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)): `toolbar` in `<user data>/workspaces/<id>.toml`, and the labels app-wide in `settings.toml`:

  ```toml
  # workspaces/default.toml: the toolbar is one key of the workspace's whole file, beside its tabs
  version = 1
  toolbar = ["menu-button", "import", "export", "reset", "undo", "redo", "space",
             "copy-edit", "apply-edit", "flexible-space", "cursors", "flexible-space",
             "jobs", "save-state"]

  [[tabs]]
  id = "photos"
  # … the rest of its tabs, then [pinned] and [order] (decision 96)

  # settings.toml
  [toolbar]
  labels = true
  ```

- A workspace whose file has no `toolbar`, a built-in nobody changed among them, follows each build's default toolbar, so later default items reach anyone who never changed theirs. Only a change to the toolbar writes `toolbar`, the whole list as it then is; a write for a change to the tabs leaves a missing list missing.
- An id this build does not know, a tool or an action removed or from a newer build, is skipped and kept in the file.
- A new workspace from the visual editor copies the list, and Workspace ▸'s Restore Defaults, deleting a built-in's file, puts the default back.
- **Customise Toolbar…**, from the toolbar menu and View › Panels, opens a dedicated window of the dialog host ([0041](0041-dialogs-as-native-modal-windows.md)), built much later; its design is settled when it is built. There is no palette inside the main window.

### The overflow

The toolbar is measured every frame at the current scale and language, and gives way one step at a time. A step is undone only once 16 points are spare.

1. The labels fold, all at once.
2. The fixed spaces narrow to 8 points, then go one at a time from the end, so that any number of them fits.
3. The actions move into » (`toolbar-more`, 30 points) one at a time:
   - in reverse reading order, tools' items before the app's, Undo and Redo last;
   - » stands just before the first flexible space, or, with none in the list, after the last item, at the toolbar's end before the caption buttons.
4. The status words fold to their signs.
5. Undo and Redo go into ».

- » lists its items in the toolbar's order, with their keys, check marks and disabled states. It replaces the built ⋯ More actions and matches the Tools panel's ».
- **Never folded:** the parts placed, the cursors' well among them, », the caption buttons and 24 points of empty area, counted on both sides of the well together.
- **When even those do not fit,** at a large interface size, the window's minimum width grows to their width, so the bar never overlaps itself or the caption buttons and keeps its 24 points to move the window.
- A test checks at 500 points, in every language, both platforms' defaults and a list with every part placed.

### Labels

- **Labels by default, under the icons,** as the Tools panel's tab names are under theirs. Toolbar Labels is one app-wide setting, `toolbar.labels`, on by default. It is in View › Panels, the toolbar menu and Settings › General. Off, each item is its icon alone, and the Toolbar keeps its height.
- **A labelled item:** in the 44-point Toolbar, 6 points above the 16-point icon in `text_secondary`, 3 points, then the word under it, centred, at the caption size (`text.caption`, 11 points), Regular, in `text`, as the tab names are set. The item is as wide as its word or its icon, the wider, with 8 points at each side. The Toolbar keeps its 44 points: the bar, the traffic lights and the caption buttons are the same with labels on or off.
- **The word** is the action's message's new `.toolbar` attribute where it has one, else the message: short, one line. A test fails on a word over 96 points at the caption size. Ukrainian's Copy edit and Apply edit need a `.toolbar` form.
- **Tooltips** always give the words and the first key ("Undo Exposure · Ctrl+Z"), labels on or off. A tool's item is prefixed by its tool's title ("Orientation: Rotate 90° clockwise").

### States

| State | How it looks and acts |
| --- | --- |
| Rest, under the pointer, pressed, focused | An `IconButton` ([0081](0081-icon-only-buttons-draw-no-plates-on-is-the-filled-icon.md)), the focus ring after keys |
| Disabled: no photo, a sheet or dialog open, not live, view-only | `text_disabled`, the tooltip saying why; not an empty area |
| Not built yet | Disabled, its tip "To come" |
| On (Before and After, Clipping Warnings) | The filled icon in `text_strong`, an AccessKit toggle, pressed |
| A tool's item, the tool off | Enabled; pressing it turns the tool on |
| A tool the workspace leaves out | Enabled; it runs as its button would |
| A background-only tool while a layer is edited (Phase 4) | Enabled; it acts on the background, and the tooltip says so |
| A tool or an action gone in a later build | Not drawn; its id kept in the file |

### Keys and scripts

- **F6** into the toolbar lands on its first control: by default the menu button on Windows and Linux and the first action that can run on the Mac, Import… where it can. With nothing focusable it skips the region.
- **The menu button, shown, never folds,** in either of its drawings.
- **Scripts** see nothing new: an item sends the command its action sends.

## Options considered

- **A fixed Toolbar, as built.** No room for a person's own actions or a tool's.
- **A palette inside the main window,** from which items are dragged onto the toolbar. Drags on a title bar compete with moving the window, and it is a second mode of the main window. A dedicated window is one place, and it can wait while the file serves.
- **F6 landing on the toolbar itself when nothing in it is focusable,** with Shift+F10 there opening the toolbar menu. A focus stop with no control is a new kind of stop for one case; Show Menu Button in View › Panels uses the route F10 already keeps.
- **A limit on fixed spaces.** A number would be arbitrary and still depend on the window's width; removing them as the bar narrows fits any number.
- **Keeping the system's window menu on a right-click on empty space.** The toolbar menu would then open only on items, so a toolbar with every item hidden could never be reached from it, and one gesture would mean two things on one bar. The system's key keeps its menu.
- **One toolbar for the app, or one per layout.** A toolbar suits a task as a workspace's tabs do (83). A layout is what the Viewer shows, not a task.
- **Labels per workspace, or per item.** Labels are about reading the bar, not about the task. One setting is simpler and folds as one step.
- **The word beside its icon,** 6 points after it at the body's size. Each item is wider, so the bar folds sooner, and it reads unlike the Tools panel's tab names, which sit under their icons.
- **A setting for the word's place, under or beside.** Two looks double the fold widths, their tests and the snapshots in every language for little gain; the Tools panel's tab names have one look.
- **Icons alone by default.** The words help a person new to the app, as the Tools panel's tab names, on by default, do; they are the first thing to fold as the window narrows, and the tooltips keep them.
- **Processing tools' items without a limit.** The overflow and the bar would fill with one tool's buttons.
- **A utility held to two items as well.** History owns six of the actions people use on every photo, Reset, Undo, Redo, Copy edit, Apply edit and Copy or Apply Parts…; a limit would leave some of them the app's, apart from the tool they belong to.
- **The edit's actions kept the app's own,** Export… and Undo among them. An action would then be offered apart from its tool, and a third-party tool's buttons would work otherwise than a built-in one's.
- **Separator lines between groups.** The Toolbar parts its groups by space, and a line on a title bar reads as a border.

## Consequences

- **Decision 39:**
  - a right-click on the Toolbar's empty areas opens the toolbar menu on every platform, and the system's window menu stays on the system's own key;
  - `Toolbar` reports the right-click with what is under it;
  - the menu button may be hidden, never folds while shown, and F6 lands on the toolbar's first control.
- **Decision 91:** the Toolbar is an app-only place, `Toolbar`, holding the toolbar menu.
- **Decisions 83 and 96:** a workspace also holds its `toolbar` list, in its file.
- **Decision 36:** a registry action that belongs to no tool may carry an icon, which makes it a candidate, and its message a `.toolbar` word.
- **`apertrust-ui`:** `Toolbar` draws a list of items, with spaces, the overflow and labels, in place of its fixed parts, and » in place of ⋯ More actions. It knows parts and items only as plain data.
- **The app:**
  - the placeable set, the parts, the app's own actions, the tools' items and the spacings, which the Toolbar and the Customise Toolbar window read;
  - the list from the workspace in place of `TOOLBAR`;
  - the guards;
  - the toolbar menu;
  - `toolbar.labels` in the settings schema;
  - Show Menu Button in View › Panels on Windows and Linux;
  - the window's minimum width raised to what the toolbar never folds;
  - the tests: of the overflow at 500 points, of the parts' ids, of the declared actions and the icon names, and of the words' width.
- **`apertrust-tools`:** `ToolbarItem` and `ToolDefinition::toolbar`, with their checks.
- **The cursors' well** ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)) is a part of the list, `cursors`, by default between two flexible spaces; the built narrowing folds the actions before it.
- **Windows' native hit testing** (planned, `apertrust-display`): it takes a right-click on an `HTCAPTION` rectangle itself (`WM_NCRBUTTONUP`) and reports it, so Windows does not open its menu there.
- **The default changes:** every item shows its word under its icon by default, and the narrowing's thresholds change with it.
