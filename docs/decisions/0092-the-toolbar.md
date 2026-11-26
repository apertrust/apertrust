# 0092. The Toolbar

## Context

The Toolbar is each main window's title bar ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)). It is drawn from the workspace's list of item ids, after the menu button on Windows and Linux, by default Import…, Export…, Reset, Undo, Redo, the cursors' well, Guides, Copy edit and Apply edit, the jobs spinner and the save state. As it narrows it folds the actions into » one at a time.

People want the actions they use on every photo within reach, and tools want buttons of their own there, such as Orientation's rotations. Several things constrain how:

- The window must stay movable by the empty parts of the bar.
- The caption buttons and the Mac's traffic lights are the platform's.
- Every command is a registry action ([0036](0036-the-design-system.md)).
- Tools add items as data and run no code in the interface ([0091](0091-context-menus-places-and-tools-items-as-actions.md)).
- What a person arranges belongs to the workspace, with the workspace's file always a way to change it ([0083](0083-the-side-panels-layout-dragged-and-floating.md), [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)).

## Decision

**The Toolbar is one a person customises.** It is an ordered list of item ids, kept for each workspace and drawn by the app from the registry. Everything but the menu button, the caption buttons and the Mac's traffic lights may be moved, removed or added, in the customise mode alone, within the Toolbar itself, or in the workspace's file. The menu button is fixed at the Toolbar's start on Windows and Linux, no item of the list. Built: tools' items, the set of what can be placed, and the list, kept in each workspace's file and drawn from that set, with the guards but the assistant's, the overflow with the window's minimum width, the labels with their setting, the toolbar menu with View › Panels' items, and an action's own menu, Guides' the first, and the customise mode (below).

### The list and its defaults

There is one ordered list of item ids for each workspace. The menu button, the caption buttons and the traffic lights are not in it: they stay where they are drawn, and the toolbar fills the room between its start inset, or the menu button, and its end.

| Platform | The default list, today's order |
| --- | --- |
| macOS | (the traffic lights) `import`, `export`, `cull` (Phase 3), `reset`, `undo`, `redo`, `flexible-space`, `cursors`, `jobs`, `assistant` (Phase 6), `save-state`, `flexible-space`, `guides`, `space`, `copy-edit`, `apply-edit` |
| Windows, Linux | (the menu button) the same, then 16 points (`spacing.xl`) before minimise, maximise and close |

- There is one list for every platform.
- **The menu button** stands first on Windows and Linux, before every item, whatever the list holds, drawn as the app's icon or ☰ as the setting Menu button, `toolbar.menu_button`, says ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)). It is the one way to the menus by the pointer there, so nothing moves, removes or adds it, the workspace's file and the customise mode alike; the Mac draws none. A file's `menu-button`, written by an older build, is an id this build does not know: skipped where it stands, and kept in the file.
- Before and After, Clipping Warnings and Zoom to 100% are offered but not in the default, since the viewer bar has them. So is Settings….
- **The cursors' well,** `cursors`, stands between two flexible spaces at the start of the middle group, the jobs spinner and the save state after it, which they centre in the room between the actions at the start and Guides, Copy edit and Apply edit at the end ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md), [cursors.md](../cursors.md#the-well)). Hidden, it leaves every cursor with a key reachable by it, and every cursor's work by its tool's own button, so a cursor with no key, as Exposure's, waits for the well or a key the person gives it: Select on Esc, Pan on A held, a picker's work by its tool's picker button, a mask's by the Layers section's buttons, Copy and apply's by Copy edit and Apply edit.

**What can be placed:**

| Kind | Ids | How many |
| --- | --- | --- |
| Parts | `jobs`, `save-state`, `assistant` (Phase 6), and `cursors`, the whole well of cursors, every group in it ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)) | One each |
| The app's actions | Only actions that belong to no tool, pressed or toggled, to which the registry gives an icon: Guides, Open…, Open Folder…, Settings…, Full Screen, Zoom to 100%, Zoom to Fit, Jobs (`show-jobs`), Next Photo and Previous Photo; Cull, Before and After, Clipping Warnings and Show in Folder as their actions are built. Import… is here until the Import tool, which then declares it | One each |
| Tools' actions | A tool's own items, which it declares (Tools' items, below): a processing tool's `tool-<tool>-<action>`, generated from its own actions as decision 91 built them, or a utility's registry action, as History's `undo`; the item's id is the action's | One each |
| Spacing | `space`, 16 points; `flexible-space`, sharing what is left | Any number; the overflow removes fixed spaces as it narrows |

- **Never placeable:** hold actions (H), focus moves (F6), the members of a set (the backgrounds, the workspaces), and the platform's own items.
- **No separator line:** groups are told apart by space alone.
- **Parts' ids are their own.** A test fails if a part's id is a registry action's id.

**Tools' items.** An action that belongs to a tool is offered for the toolbar by that tool, in its definition, so a tool's buttons travel with it, a third-party tool's included; only actions that belong to no tool are the app's.

- A tool offers actions with `ToolDefinition::toolbar: &'static [ToolbarItem]`, in the parent every tool's definition embeds, a processing tool's and a utility's alike ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)), where `ToolbarItem { action, icon }`:
  - `action`, for a processing tool, names one of its own actions as its menu items do, `rotate_ccw`, which the registry holds as `tool-<tool>-<action>`; for a utility, the registry action it names, as History's `reset` in its menus;
  - `icon` names one of `apertrust-ui`'s icons, `export`, or is none for an icon `Action` end item's action, which takes its button's icon ([0107](0107-packed-lines-one-line-row-with-end-items.md)).
- The definition's check refuses an empty action, an action a processing tool does not have, the same action twice, no icon on an action that is no icon end item's, a utility's always, and more than two items on a processing tool; a utility has no limit. App tests fail on a declared action the registry does not hold and on an icon name `apertrust-ui` does not have.
- Offering an action only makes it a candidate: it is on the toolbar only where a person places it.
- The app lists every candidate in one stable order, the parts, the app's own actions, every tool's items in the tools' order, then `space` and `flexible-space`, each with its id, its kind, its icon and its words' id: the Toolbar draws from that list and the customise mode's Add ▸ lists it.

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

The app's own, with the icons the registry gives them: `import` (`import`, until the Import tool), `guides` (`guides`), `open` (`photo`), `open-folder` (`folder-open`), `settings` (`settings`), `full-screen` (`maximise`), `zoom-100` (`zoom-in`), `zoom-fit` (`zoom-out`), `show-jobs` (`process`), `next-photo` (`chevron`) and `previous-photo` (`back`).

**Guards.** A status part can be hidden only in its quiet states, and what it guards still works:

| Hidden | What still holds |
| --- | --- |
| The save state | A failed write still shows its warning and Not saved at the toolbar's end; the message over the photo, the live region and the quit alert are unchanged |
| The jobs | The spinner shows while a failed job is left; View › Jobs and Ctrl+Shift+J still work |
| The assistant | It shows whenever an assistant is connected |
| The cursors | Every cursor keeps its action and key, and its work its tool's own button; the cursor chosen stays chosen |
| Everything | The toolbar is all empty area, the menu button apart, and still moves the window |

**Moving the window.** These are empty areas: they move the window and take a double-click, as the Toolbar's empty areas do (39):

- flexible spaces;
- the parts of a fixed space outside egui's 5-point margin round a control;
- a status slot showing nothing.

The overflow always keeps 24 points of them. On Windows the hit map's `HTCAPTION` rectangles come from the toolbar's layout, the items are `HTCLIENT`, and `HTMAXBUTTON` stays over the maximise button, so the snap layouts flyout is unchanged.

### The toolbar menu

**A right-click anywhere on the toolbar opens the toolbar menu,** on an item or on empty space, the caption buttons apart, and the menu button while the Toolbar is the title bar, on every platform, as does Shift+F10 or the Menu key on a focused item or on the menu button, which opens it as on empty space. It is an app-only place of decision 91's, `Place::Toolbar`. It holds:

1. Toolbar Labels, checked while on;
2. Customise Toolbar…, which puts the main window into the customise mode, the item it opened on selected (The customise mode, below).

**Every item is reachable from the keys,** as decision 91 asks, whatever the toolbar shows: both are in View › Panels, which F10 opens on Windows and Linux, and the system's Ctrl+F2 reaches in the Mac's menu bar. A toolbar with nothing focusable needs no keyboard route of its own, since nothing in its menu acts on it alone. Nothing in the toolbar menu changes the arrangement: moving, adding, removing and Reset Toolbar happen in the customise mode alone.

**The system's window menu** is no longer on a right-click on the toolbar's empty space on Windows and Linux. It stays on the system's own key, Alt+Space on Windows and GNOME, and on a right-click on the menu button while the toolbar is the title bar, as on a title bar's icon; X11 still has none, and the Mac's conventions are unchanged.

### An action's own menu

**An action may have a menu of its own on the Toolbar,** where a few choices are worth having one click away; most have none. Guides' is the first: a click turns the guides on or off, and its menu chooses their type, as darktable's guides button does. Built.

- **Declared as data,** as decision 91's menus are, in the app's list of the Toolbar's actions with a menu, `menus::ACTION_MENUS`: its own registry actions, and a setting's choices as a checked list, each choice an action of its own, the one in use checked; a submenu one level deep. The app builds the menu, and only the action chosen runs.
- **Opened** by a right-click, a Control-click on the Mac, Shift+F10, the Menu key or a screen reader's ShowContextMenu on that item, labels on or off.
- **It holds** its own items first, a separator, then the toolbar menu's entries, so the toolbar menu stays reachable from every item.
- **An item without one** opens the toolbar menu, as a right-click anywhere else on the Toolbar does.
- **In »,** such an item is a submenu under its words: the action itself, checked while on, a separator, then its own items, so a folded item still runs with a click.
- **Every item is reachable elsewhere,** as decision 91 asks: Guides' in View › Guides ▸, which holds the action, a separator, its types, a separator and Flip ▸.
- **No badge** marks the item. Its tooltip names the menu on a line of its own after its words and keys: "Guides (G)", then "Right-click for the guide type".

**Guides' menu:** the ten guide types of darktable's reference release (`src/gui/guides.c`), the one in use checked: Grid, Rules of thirds, Metering, Perspective, Diagonal method, Harmonious triangles, Golden sections, Golden spiral, Golden spiral sections and Golden mean (all guides); a separator; Flip ▸, None, Horizontally, Vertically and Both, drawn disabled while the type is one darktable never flips. Choosing a type turns the guides on. The type and the flip are the settings `guides.type` and `guides.flip`, at darktable's defaults, Rules of thirds and None. `guides` is one of the app's own actions, placeable with an icon of its own and checked while on, by default right of the cursors' well, a space before Copy edit, and not on the viewer bar.

### Customising

- **The workspace's file** is a full way to customise beside the customise mode, and stays one (83, [0096](0096-workspaces-as-files-hidden-tools-and-finding-a-tool.md)): `toolbar` in `<user data>/workspaces/<id>.toml`, and the labels app-wide in `settings.toml`:

  ```toml
  # workspaces/default.toml: the toolbar is one key of the workspace's whole file, beside its tabs
  version = 1
  toolbar = ["import", "export", "reset", "undo", "redo", "flexible-space",
             "cursors", "jobs", "save-state", "flexible-space", "guides", "space",
             "copy-edit", "apply-edit"]

  [[tabs]]
  id = "photos"
  # … the rest of its tabs, then [pinned] and [order] (decision 96)

  # settings.toml
  [toolbar]
  labels = true
  ```

- A workspace whose file has no `toolbar`, a built-in nobody changed among them, follows each build's default toolbar, so later default items reach anyone who never changed theirs. Only a change to the toolbar writes `toolbar`, the whole list as it then is; a write for a change to the tabs leaves a missing list missing.
- An id this build does not know, a tool or an action removed or from a newer build, is skipped and kept in the file.
- **Reset Toolbar,** the registry's `reset-toolbar`, in the customise mode alone, makes the draft the default; Done then takes `toolbar` out of the file as a Toolbar change writes it, read again first, the tabs, `pinned`, `order`, `hidden` and every other key kept as the file has them; a built-in's file then holding nothing its bundled file does not stays as it is. Named after the tab bar's Reset Tab (96).
- A new workspace from the visual editor copies the list, and Workspace ▸'s Restore Defaults, deleting a built-in's file, puts the default back.
- **Customise Toolbar…**, from the toolbar menu and View › Panels, puts the main window into the customise mode (below).

### The customise mode

**Every change to the arrangement happens only in the customise mode,** or in the workspace's file: moving, adding and removing. At rest the toolbar menu and View › Panels hold Toolbar Labels and Customise Toolbar… alone: the arrangement, and its reset, belong only to the mode.

**The mode stays within the Toolbar.** Customise Toolbar… puts the main window into it, with no palette and no panel, for the workspace shown alone. It is a modal state, `State::modal` and `State::dialog` set as under About, with one exception to the modal gate: the Toolbar's region and its menus take input. So it is not `egui::Modal`, which blocks everything under it: the rest of the window, the Tools panel, the Viewer, the Browser and any floating panel, dims under `scrim` on an `Area` of its own and takes no input, and every action that opens a dialog, the view's keys, View › Workspace, Open… and the photo's Undo rest. Jobs, saving and the engine keep running. Customise Toolbar… is disabled while a modal is up, so the mode never opens over a dialog.

**The Toolbar becomes editable:**

- each item on a `raised` plate, `button` under the pointer and `accent_subtle` selected; the spacings as `sunken` tiles, a flexible space with Lucide's `move-horizontal` and a fixed space a 16-point tile with a bar; the parts that are invisible at rest with faces, the still spinner and "Saving…"; every item drawn enabled and at rest, since disabled and on states are the photo's;
- the items wiggle once as the mode starts, a translation of up to 1.5 points over `wiggle_ms` (360 ms), none with reduced motion or the motion tokens at zero;
- Done, a primary text button, at its end, outside what scrolls;
- at most one item selected, by a click, a right-click or ← and →.

**The menu button** (Windows and Linux) stays fixed at the start, outside the list: drawn as at rest with no plate, it does not wiggle, is never selected, dragged, moved, removed or listed in Add ▸, and takes no click, the menus resting as under any modal. A right-click on it opens the customise menu as at the list's start, so Add ▸ adds first.

**The customise menu,** on a right-click on an item, which selects it, the cursors' well and the spacing tiles included, or on empty space, a Control-click on the Mac, or Shift+F10, the Menu key or ShowContextMenu on the focused Toolbar: Move Left, Move Right, Remove from Toolbar (on an item alone), —, Add ▸, —, Reset Toolbar, Toolbar Labels, —, Done. Add ▸ lists only what the draft does not hold, in the set's order: the parts and the app's own actions, a submenu per tool with an item left, titled by the tool's title, then Space and Flexible Space, always. Its items are registry actions, as decision 91 asks: Move Left, Move Right, Remove from Toolbar and Done are actions available only while customising, with no default keys; `reset-toolbar` keeps its id, available only while customising and disabled, saying why, while `--conf` sets the workspace, and `toolbar-labels` keeps its id; both act on the draft in the mode; Add ▸'s items are compound ids, an action and the placeable's id joined, as Add Tool ▸'s are. Never on the caption buttons, the traffic lights or Done.

**Moving** is also by a drag along the Toolbar, a press moving 6 points, with an insertion line at the nearest gap, and by Alt+← and Alt+→ (Option on the Mac). Every drag has a single-pointer way, the menu's Move Left and Move Right, and a key, so WCAG 2.2's 2.5.7 is met. A release off the Toolbar does nothing. Delete and Backspace remove.

**No window gestures while customising:** no part of the Toolbar moves the window or takes a double-click. The caption buttons and the traffic lights stay live; the resize bands too.

**It is a draft,** of the list, the labels and whether the list is Reset's:

- Esc (Cmd+. too on the Mac) cancels, as do closing the window and Quit, which then close or quit as they would: nothing is written.
- Done, the button, the menu's Done or Enter, writes once: `Move::ResetToolbar` after a Reset, the list still the default, where the file holds a list; else nothing where the list is unchanged; otherwise `Move::Toolbar` with the whole list. `toolbar.labels` is written only if the labels were flipped. None of it is an edit.
- An id this build does not know is anchored to the known id before it at the mode's start and written back after that anchor, or after the nearest earlier known id still there, or at the start; an old list's `menu-button` is dropped. A Reset drops them with the list.

**While customising, the Toolbar never folds.** It is one line that scrolls sideways between its start and Done, the flexible spaces shrinking to 40-point tiles, with a fold mark, a small », on what » would take at rest at this width and on each fixed space it would drop; » itself is drawn as a face before Done. `Toolbar::least_width` still records the at-rest fold, so the window's minimum width does not jump.

**Screen readers** meet one modal `Dialog` named with the workspace, holding the Toolbar as a horizontal `ListBox` whose options carry their names, positions, selection and fold marks, with Move Left, Move Right and Remove from Toolbar as custom actions, and Done a default button; the Toolbar's `Toolbar` role comes back as the mode ends. Entering, each change and leaving are announced politely.

### The overflow

The toolbar is measured every frame at the current scale and language, and gives way one step at a time. A step is undone only once 16 points are spare. The steps are the same with labels on or off: when items do not fit, » appears and the items that do not fit go into its menu, a labelled item with its word, the ones still shown keeping theirs.

1. The fixed spaces narrow to 8 points, then go one at a time from the end, so that any number of them fits.
2. The actions move into » (`toolbar-more`, 30 points) one at a time:
   - from the toolbar's end, in reverse reading order across the whole bar, the rightmost first, so the end's group goes before the start's; within a group, between the same flexible spaces, tools' items before the app's; Undo and Redo last;
   - » stands at the toolbar's end, after the last item drawn: before the caption buttons on Windows and Linux, at the bar's right end on the Mac; never in the middle. The flexible spaces still centre the middle group in the room left.
3. The status words fold to their signs.
4. Undo and Redo go into ».

- » lists its items in the toolbar's order, with their keys, check marks and disabled states. It matches the Tools panel's ».
- **Never folded:** the menu button, the parts placed, the cursors' well among them, », the caption buttons and 24 points of empty area, counted on both sides of the well together.
- **When even those do not fit,** at a large interface size, the window's minimum width grows to their width, so the bar never overlaps itself or the caption buttons and keeps its 24 points to move the window.
- A test checks at 500 points, in every language, both platforms' defaults and a list with every part placed.

### Labels

- **Labels by default, under the icons,** as the Tools panel's tab names are under theirs. Toolbar Labels is one app-wide setting, `toolbar.labels`, on by default. It is in View › Panels, the toolbar menu and Settings › General. Off, each item is its icon alone.
- **The Toolbar's height:** 44 points with labels off, the themes' `toolbar`; 62 with labels on, the themes' `toolbar_labelled`. The height follows the setting alone, never the window's width.
- **An action's icon is 22 points,** labels on or off, as the Tools panel's tabs' icons: every `ToolbarButton`, the app's actions and tools' items alike, drawn by `Icon::paint_stroked` at that size with a 1.875-point line (`ACTION_STROKE`), half as heavy again as every other icon's 1.25, from the same files. An icon that keeps a box of its own, Full Screen's, keeps its size.
- **Labels off,** each action is a 32-point button with its icon at its centre.
- **A labelled item:** 11 points from the bar's top to the 22-point icon in `text_secondary`, 4 points, then the word on its line, centred, at the caption size (`text.caption`, 11 points, a 14-point line), Regular, in `text`, as the tab names are set, and 11 points below it, the margins the icon has above and below without labels. The item is as wide as its word or its icon, the wider, with 8 points at each side.
- **Everything else keeps its size, centred in the bar,** labels on or off: the menu button, the cursors' well and its buttons, », the jobs spinner, the save state, the caption buttons, which run its whole height, and the Mac's traffic lights.
- **The word** is the action's message's new `.toolbar` attribute where it has one, else the message: short, one line. A test fails on a word over 96 points at the caption size. Ukrainian's Copy edit and Apply edit need a `.toolbar` form.
- **Tooltips** always give the words and the first key ("Undo Exposure · Ctrl+Z"), labels on or off. A tool's item is prefixed by its tool's title ("Orientation: Rotate 90° clockwise").

### States

| State | How it looks and acts |
| --- | --- |
| Rest, under the pointer, pressed, focused | An `IconButton` ([0081](0081-icon-only-buttons-draw-no-plates-on-is-the-filled-icon.md)), the focus ring after keys |
| Disabled: no photo, a sheet or dialog open, not live, view-only | `text_disabled`, the tooltip saying why; not an empty area |
| Not built yet | Disabled, its tip "To come" |
| On: any placed action whose menu item is checked, as Guides while the guides are on | The filled icon in `text_strong` where the icon has one, its colour alone where it has not, as Guides'; an AccessKit toggle, pressed; under », its item checked |
| A tool's item, the tool off | Enabled; pressing it turns the tool on |
| A tool the workspace leaves out | Enabled; it runs as its button would |
| A background-only tool while a layer is edited (Phase 4) | Enabled; it acts on the background, and the tooltip says so |
| A tool or an action gone in a later build | Not drawn; its id kept in the file |

### Keys and scripts

- **F6** into the toolbar lands on its first control: the menu button on Windows and Linux, and on the Mac by default the first action that can run, Import… where it can. With nothing focusable it skips the region.
- **The menu button never folds,** in either of its drawings.
- **Scripts** see nothing new: an item sends the command its action sends.

## Options considered

- **A fixed Toolbar, as built.** No room for a person's own actions or a tool's.
- **A dedicated window of the dialog host, holding a copy of the Toolbar.** A drag cannot cross from the dialog host's window into the main window, so every drag would land on a copy, and the person would watch two drawings of one list.
- **A palette hanging under the Toolbar inside the window,** from which items are dragged onto it. More visible, but bigger to build, a palette, a hanging panel and a drag across two surfaces, with no single-pointer way. That no part of the Toolbar moves the window in the mode, and that the mode is one more modal state like About, answer the drag on a title bar and a second mode of the main window.
- **A dialog with two lists,** what is placed and what can be. Abstract: a vertical list standing for a horizontal bar.
- **Reset Toolbar at rest,** in the toolbar menu and View › Panels. It changes the arrangement outside the mode, all at once, with no draft for Esc to cancel.
- **Hide on the toolbar menu at rest,** beside the mode. Arrangement would then happen in two places, one of them without Add.
- **F6 landing on the toolbar itself when nothing in it is focusable,** with Shift+F10 there opening the toolbar menu. A focus stop with no control is a new kind of stop for one case; View › Panels holds the toolbar menu's items on the route F10 already keeps.
- **The menu button an item of the list,** moved or hidden as the others are, with Show Menu Button to put it back. On Windows and Linux it is the one way to the menus by the pointer, so hiding it is never what a person wants, and a guard and an action to restore it would serve that one case alone.
- **A limit on fixed spaces.** A number would be arbitrary and still depend on the window's width; removing them as the bar narrows fits any number.
- **Keeping the system's window menu on a right-click on empty space.** The toolbar menu would then open only on items, so a toolbar with every item hidden could never be reached from it, and one gesture would mean two things on one bar. The system's key keeps its menu.
- **One toolbar for the app, or one per layout.** A toolbar suits a task as a workspace's tabs do (83). A layout is what the Viewer shows, not a task.
- **Labels per workspace, or per item.** Labels are about reading the bar, not about the task. One setting is simpler and folds as one step.
- **The word beside its icon,** 6 points after it at the body's size. Each item is wider, so the bar folds sooner, and it reads unlike the Tools panel's tab names, which sit under their icons.
- **A badge or a chevron on an item with a menu of its own,** as the cursors' well's groups have. It widens the item and sets it apart from its neighbours for a menu the tooltip can name, as darktable's guides button's does.
- **The action's menu in place of the toolbar menu on its item.** Toolbar Labels and Customise Toolbar… would then need another way from the pointer there.
- **darktable's popover of guide settings on the right-click.** A menu of registry actions keeps every choice reachable from the menu bar and the keys; the overlay's colour, its contrast and the grid's lines come later, in Settings or a popover.
- **The labels inside the 44-point bar,** 6 points above the icon and 3 below it. It squeezes icon and word together and leaves no room below the word.
- **Action icons at 16 points,** the viewer bar's size. Beside the Tools panel's 22-point tab icons they read small.
- **New 22-point icon files,** as `toolbar-more` beside `more`. The icons render at any size from Lucide's grid with the same line, so a second file would only repeat the first.
- **» before the first flexible space.** The overflow sat mid-bar, and the start group folded while the end stayed.
- **The labels folding first,** all at once, before any item goes under ». The Toolbar would behave one way with labels and another without; one rule serves both: what does not fit goes under ».
- **A setting for the word's place, under or beside.** Two looks double the fold widths, their tests and the snapshots in every language for little gain; the Tools panel's tab names have one look.
- **Icons alone by default.** The words help a person new to the app, as the Tools panel's tab names, on by default, do, and the tooltips keep them.
- **Processing tools' items without a limit.** The overflow and the bar would fill with one tool's buttons.
- **A utility held to two items as well.** History owns six of the actions people use on every photo, Reset, Undo, Redo, Copy edit, Apply edit and Copy or Apply Parts…; a limit would leave some of them the app's, apart from the tool they belong to.
- **The edit's actions kept the app's own,** Export… and Undo among them. An action would then be offered apart from its tool, and a third-party tool's buttons would work otherwise than a built-in one's.
- **Separator lines between groups.** The Toolbar parts its groups by space, and a line on a title bar reads as a border.

## Consequences

- **Decision 39:**
  - a right-click on the Toolbar's empty areas opens the toolbar menu on every platform, and the system's window menu stays on the system's own key and on a right-click on the menu button;
  - `Toolbar` reports the right-click with what is under it;
  - the menu button is fixed at the Toolbar's start, no item of the list, and never folds, and F6 lands on the toolbar's first control.
- **Decision 91:** the Toolbar is an app-only place, `Toolbar`, holding the toolbar menu.
- **Decisions 83 and 96:** a workspace also holds its `toolbar` list, in its file.
- **Decision 36:** a registry action that belongs to no tool may carry an icon, which makes it a candidate, and its message a `.toolbar` word; the themes have a size for the labelled Toolbar, `toolbar_labelled`.
- **Decision 39:** the Mac's traffic lights are centred in the Toolbar at whichever height it has, so the close button's centre is 31 points from the window's left and top edges with labels on and 22 with them off.
- **`apertrust-ui`:** `Toolbar` draws a list of items, with spaces, the overflow and labels, in place of its fixed parts, and » in place of ⋯ More actions. It knows parts and items only as plain data. `Toolbar::height` gives its height for the labels setting, which the app sizes the Toolbar's panel and places the traffic lights by; `IconButton::icon_size` draws an action's icon at 22 points.
- **The app:**
  - the placeable set, the parts, the app's own actions, the tools' items and the spacings, which the Toolbar and the customise mode read;
  - the list from the workspace in place of `TOOLBAR`;
  - the guards;
  - the toolbar menu; Reset Toolbar, `reset-toolbar`, in the customise mode, and `layout::Move::ResetToolbar`;
  - `toolbar.labels` in the settings schema;
  - Toolbar Labels and Customise Toolbar… in View › Panels on every platform;
  - the window's minimum width raised to what the toolbar never folds;
  - the tests: of the overflow at 500 points, of the parts' ids, of the declared actions and the icon names, and of the words' width.
- **`apertrust-tools`:** `ToolbarItem` and `ToolDefinition::toolbar`, with their checks.
- **An action's own menu** (built): in the app, `menus::ActionMenu` in `ACTION_MENUS`, its own menu and the one » shows, built with the toolbar menu's entries after it, with a test that every item is reachable elsewhere; the `guides` action, an action for each guide type and flip, View › Guides ▸, and `guides.type` and `guides.flip` in the settings schema. In `apertrust-ui`, `ToolbarButton::menu`, shown as a submenu in », its tooltip with the menu's line (`IconButton::tip_line`). Decision 91's Toolbar place holds it.
- **The cursors' well** ([0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)) is a part of the list, `cursors`, by default between two flexible spaces; the built narrowing folds the actions before it.
- **Windows' native hit testing** (planned, `apertrust-display`): it takes a right-click on an `HTCAPTION` rectangle itself (`WM_NCRBUTTONUP`) and reports it, so Windows does not open its menu there.
- **The customise mode** (built): in `apertrust-ui`, the Toolbar's edit state, reporting what was done as data, with the motion token `wiggle_ms` and Lucide's `move-horizontal`; in the app, the draft beside `toolbar.rs`, the scrim as its own `Area`, the customise menu as menu data, its registry actions, and `customise-toolbar` enabled; the toolbar menu's Hide and its action `toolbar-hide` gone.
- **The default changes:** every item shows its word under its icon by default, and the narrowing's thresholds change with it.
