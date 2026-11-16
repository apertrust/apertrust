# 0092. The toolbar

## Context

The top panel is each main window's title bar ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)). Today it holds a fixed set of parts, from a constant in the app (`TOP_PANEL`): the ☰ on Windows and Linux, the mark, Import…, Export…, Reset, Undo, Redo, Copy edit and Apply edit, the jobs spinner and the save state. Under 960 points it folds them into ⋯ More actions.

People want the actions they use on every photo within reach, and tools want buttons of their own there, such as Orientation's rotations. Several things constrain how:

- The window must stay movable by the empty parts of the bar.
- The caption buttons and the Mac's traffic lights are the platform's.
- Every command is a registry action ([0036](0036-the-design-system.md)).
- Tools add items as data and run no code in the interface ([0091](0091-context-menus-places-and-tools-items-as-actions.md)).
- What a person arranges belongs to the workspace, with the settings file always a way to change it ([0083](0083-the-side-panels-layout-dragged-and-floating.md)).

## Decision

**The top panel is a toolbar a person customises.** It is an ordered list of item ids, kept for each workspace and drawn by the app from the registry. Everything but the caption buttons and the Mac's traffic lights may be moved or hidden. Planned: Phase 3 builds the toolbar, its menu, tools' items, the overflow and labels; the Customise Toolbar window comes much later.

### The list and its defaults

There is one ordered list of item ids for each workspace. The caption buttons and the traffic lights are not in it: they stay where the platform puts them, and the toolbar fills the room between its start inset and its end.

| Platform | The default list, today's order |
| --- | --- |
| macOS | (the traffic lights) `mark`, `import`, `export`, `cull` (Phase 3), `reset`, `undo`, `redo`, `space`, `copy-edit`, `apply-edit`, `flexible-space`, `jobs`, `assistant` (Phase 6), `save-state` |
| Windows, Linux | `menu-button`, then the same, then 16 points (`spacing.xl`) before minimise, maximise and close |

- There is one list for every platform. The Mac never shows `menu-button` and never takes it out of the list.
- Before and After, Clipping Warnings and Zoom to 100% are offered but not in the default, since the central bar has them. So is Settings….

**What can be placed:**

| Kind | Ids | How many |
| --- | --- | --- |
| Parts | `menu-button` (Windows and Linux), `mark`, `jobs`, `save-state`, `assistant` (Phase 6) | One each |
| The app's actions | A registry action that is pressed or toggled and has an icon: Import…, Export…, Export Again, Cull, Reset, Undo, Redo, Copy edit, Apply edit, Copy or Apply Parts…, Open…, Open Folder…, Before and After, Clipping Warnings, Zoom to 100%, Zoom to Fit, Full Screen, Jobs, Browse, Show in Folder, Next Photo, Previous Photo, Settings… | One each |
| Tools' actions | A generated `tool-<tool>-<action>` (91) that its tool offers for the toolbar | One each |
| Spacing | `space`, 16 points; `flexible-space`, sharing what is left | Any number |

- **Never placeable:** hold actions (H), focus moves (F6), the members of a set (the backgrounds, the workspaces), and the platform's own items.
- **No separator line:** groups are told apart by space alone.
- **Parts' ids are their own.** A test fails if a part's id is a registry action's id, as `menu-button` is not the `menu` action and `mark` is not `about`.

**Tools' items.**

- A tool offers actions with `Definition::toolbar: &'static [ToolbarItem]`, where `ToolbarItem { action, icon }`:
  - `action` names one of its own actions, as `Definition::menus` does;
  - `icon` names one of `apertrust-ui`'s icons, or is none for a `Row::Buttons` action, which takes its button's icon.
- A utility offers its own with `Utility::toolbar`.
- `Definition::check` refuses an unknown action, a repeat and more than two items for a tool. An app test fails on an icon name `apertrust-ui` does not have.
- Offering an action only makes it a candidate: it is on the toolbar only where a person places it.
- The first items: Orientation's Rotate 90° anticlockwise and Rotate 90° clockwise.

**Guards.** A status part can be hidden only in its quiet states, and what it guards still works:

| Hidden | What still holds |
| --- | --- |
| The ☰ | F10, Alt alone on Windows and Alt with an access key still open the menus, hung from the toolbar's start; the toolbar menu gains Show Menu Button |
| The save state | A failed write still shows its warning and Not saved at the toolbar's end; the message over the photo, the live region and the quit alert are unchanged |
| The jobs | The spinner shows while a failed job is left; View › Jobs and Ctrl+Shift+J still work |
| The assistant | It shows whenever an assistant is connected |
| The mark | About stays in Help on Windows and Linux and in the app menu on the Mac |
| Everything | The toolbar is all empty area and still moves the window |

**Moving the window.** These are empty areas: they move the window and take a double-click, as the top panel's empty areas do (39):

- flexible spaces;
- the parts of a fixed space outside egui's 5-point margin round a control;
- a status slot showing nothing.

The overflow always keeps 24 points of them. On Windows the hit map's `HTCAPTION` rectangles come from the toolbar's layout, the items are `HTCLIENT`, and `HTMAXBUTTON` stays over the maximise button, so the snap layouts flyout is unchanged.

### The toolbar menu

**A right-click anywhere on the toolbar opens the toolbar menu,** on an item or on empty space, on every platform, as does Shift+F10 or the Menu key on a focused item. It is an app-only place of decision 91's, `Place::TopPanel`. It holds:

1. Hide "Undo", on an item;
2. Show Menu Button, while the ☰ is hidden;
3. a separator;
4. Toolbar Labels, checked while on;
5. Customise Toolbar…, drawn disabled until its window is built, its tip "Customise Toolbar comes in a later version".

**The system's window menu** is no longer on a right-click on the toolbar on Windows and Linux. It stays on the system's own key, Alt+Space on Windows and GNOME; X11 still has none, and the Mac's conventions are unchanged.

### Customising

- **The settings file** is the full way to customise until a window exists, and stays one after it (83):

  ```toml
  [workspaces.edit]
  toolbar = ["menu-button", "mark", "import", "export", "reset", "undo", "redo", "space",
             "copy-edit", "apply-edit", "flexible-space", "jobs", "save-state"]

  [top_panel]
  labels = false
  ```

- `toolbar` is written only once it changes, so a workspace nobody changed follows each build's default.
- An id this build does not know, a tool or an action removed or from a newer build, is skipped and kept in the file.
- Save as Workspace copies the list, and Reset Workspace puts the default back.
- **Customise Toolbar…**, from the toolbar menu and View › Panels, opens a dedicated window of the dialog host ([0041](0041-dialogs-as-native-modal-windows.md)), built much later; its design is settled when it is built. There is no palette inside the main window.

### The overflow

The toolbar is measured every frame at the current scale and language, and gives way one step at a time. A step is undone only once 16 points are spare.

1. The labels fold, all at once.
2. The fixed spaces narrow to 8 points.
3. The actions move into » (`top-panel-more`, 30 points) one at a time:
   - in reverse reading order, tools' items before the app's, Undo and Redo last;
   - » stands just before the first flexible space.
4. The status words fold to their signs.
5. Undo and Redo go into ».

- » lists its items in the toolbar's order, with their keys, check marks and disabled states. It replaces the built ⋯ More actions and matches the side panel's ».
- **Never folded:** the parts placed, », the caption buttons and 24 points of empty area.
- A test checks at 500 points, in every language, both platforms' defaults and a list with every part placed.

### Labels

- **Icons by default.** Toolbar Labels is one app-wide setting, `top_panel.labels`, off by default. It is in View › Panels, the toolbar menu and Settings › General.
- **A labelled item:** 8 points, the 16-point icon in `text_secondary`, 6 points, the word in the body's 13 points in `text`, 8 points.
- **The word** is the action's message's new `.toolbar` attribute where it has one, else the message. A test fails on a word over 96 points. Ukrainian's Copy edit and Apply edit need a `.toolbar` form.
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

### Keys, windows and scripts

- **F6** into the toolbar lands on its first control: by default the ☰ on Windows and Linux and the mark on the Mac. With nothing focusable it skips the region.
- **The ☰, shown, never folds.**
- **Several windows** ([0089](0089-one-instance-per-user-data-folder-a-window-per-place.md)): each window draws its current workspace's toolbar, with its own enabled states. A change is a setting sent to every window showing that workspace, and the Mac's View menu acts on the key window. `jobs` shows the instance's jobs and `save-state` its window's edit.
- **Scripts** see nothing new: an item sends the command its action sends.

## Options considered

- **A fixed top panel, as built.** No room for a person's own actions or a tool's.
- **A palette inside the main window,** from which items are dragged onto the toolbar. Drags on a title bar compete with moving the window, and it is a second mode of the main window. A dedicated window is one place, and it can wait while the file serves.
- **Keeping the system's window menu on a right-click on empty space.** The toolbar menu would then open only on items, so a toolbar with every item hidden could never be reached from it, and one gesture would mean two things on one bar. The system's key keeps its menu.
- **One toolbar for the app, or one per layout.** A toolbar suits a task as a workspace's tabs do (83). A layout is what the central panel shows, not a task.
- **Labels per workspace, or per item.** Labels are about reading the bar, not about the task. One setting is simpler and folds as one step.
- **Tools' items without a limit.** The overflow and the bar would fill with one tool's buttons.
- **Separator lines between groups.** The top panel parts its groups by space, and a line on a title bar reads as a border.

## Consequences

- **Decision 39:**
  - a right-click on the top panel's empty areas opens the toolbar menu on every platform, and the system's window menu stays on the system's own key;
  - `TopPanel` reports the right-click with what is under it;
  - the ☰ may be hidden, never folds while shown, and F6 lands on the toolbar's first control.
- **Decision 91:** the top panel is an app-only place, `TopPanel`, holding the toolbar menu.
- **Decision 83:** a workspace also holds its `toolbar` list.
- **Decision 36:** a registry action may carry an icon, which makes it a candidate, and its message a `.toolbar` word.
- **`apertrust-ui`:** `TopPanel` draws a list of items, with spaces, the overflow and labels, in place of its fixed parts, and » in place of ⋯ More actions. It knows parts and items only as plain data.
- **The app:**
  - the list from the workspace in place of `TOP_PANEL`;
  - the guards;
  - the toolbar menu;
  - `top_panel.labels` in the settings schema;
  - the tests: of the overflow at 500 points, of the parts' ids, of the icon names and of the words' width.
- **`apertrust-tools`:** `ToolbarItem`, `Definition::toolbar` and `Utility::toolbar`, with their checks.
- **Windows' native hit testing** (planned, `apertrust-display`): it takes a right-click on an `HTCAPTION` rectangle itself (`WM_NCRBUTTONUP`) and reports it, so Windows does not open its menu there.
- **The default changes:** Import…, Export…, Cull and Reset lose their words by default, labels being off, and the narrowing's thresholds change with it.
