# 0091. Context menus: places, and tools' items as actions

## Context

A right-click menu helps in a few common places: the photo, a thumbnail, and later a History step, a layer or a job's row. Tools want to add items there, History's Discard History… first, and whatever belongs to a tool should stay in that tool's code. Three things constrain how:

- `apertrust-tools` has no egui and no app types ([0028](0028-tools-bound-to-operations.md)).
- The Mac's native menus run nothing and answer with the id chosen ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)).
- Every command a person can give is an action of the registry, so that its keys, its menu items and its buttons agree ([0036](0036-the-design-system.md)).

A tool's section already has its own buttons: reset, presets and options (⋯). A slider's right-click is darktable's fine control ([0042](0042-numbers-shown-typed-and-stored.md)).

## Decision

**Context menus open only in named places, and tools add items to them as data.** Every item is a registry action, the app builds every menu, and no tool runs code in a menu.

**The places.** `Place` is a closed list.

- Open to tools' items: `Photo` (the photo in the central panel), `Thumbnail` (a grid panel cell, and a cell in Photo when several photos show), and `Options` (a tool's own ⋯, in its own section only).
- The app's alone, each built with its feature:
  - the surround (built: Background ▸);
  - the grid bar (built: Position ▸);
  - Browse's folder rows (built);
  - the grid panel's empty area: the grid bar's options, with Select All and Select None;
  - a History step: Go to Step, Show Tool, Compress up to Here and Truncate after Here;
  - the jobs window's rows: Cancel, Show in Folder and Copy Error;
  - a layer (Phase 4), the scopes, a tab's icon.
- Another place opens to tools' items only by a decision of its own.
- The top panel's empty areas never hold a menu of the app's: a right-click there is the system's window menu ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)).

**A tool takes no right-click.**

- Its header, its body and its controls open no context menu.
- Its presets come from the header's presets button.
- Its own menu is the header's options button, ⋯, a pull-down from that button.
- Shift+F10 and the Menu key on a focused header open nothing; Tab reaches ⋯.
- A control keeps its own gesture, which is never a menu: a slider's right-click opens darktable's fine control (42), and a picker button's takes an area ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)).
- The rows of a list a tool shows are items of their own and may have a menu, as a place does: Browse's folders, the Keywords list ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)), History's steps and layers. The Colour picker's stored samples keep darktable's right-click load and store (79).

**Every item is an action** of the registry (36). So it can be given keys, it is in the menu bar or its tool too, it is enabled and checked from the state, and the native menu only reports which was chosen.

- A tool's own actions become registry actions, generated once as the app starts, one for each `Row::Action` and each button of `Row::Buttons`:
  - its id is `tool-<tool>-<action>`, under which settings keep its keys;
  - its words are the action's full name, `tool-<tool>-action-<action>-full`;
  - it has no default keys and works in the view's scope;
  - it runs as its button does;
  - one whose button is drawn disabled, not built yet, is listed and never runs.
- A utility's items are the registry actions it already has, as History's Discard History… is `discard-history`.

**Tools declare their items as data.**

- A tool: `Definition::menus: &'static [MenuItem]`, where `MenuItem { place: Place, action: &'static str }` names one of the tool's own actions.
- A utility, which has no definition: `Utility::menus` in `about::UTILITIES`, each item naming the registry action that runs it.
- A tool with nothing to add declares nothing.
- `Definition::check` refuses an item that names none of the tool's actions, an item listed twice, and more than two items in one place among `Photo` and `Thumbnail`, or more than four in `Options`. An app test holds the utilities' items to the same limits and to actions the registry declares.
- A picker is no item yet: an item that picks at the pointer waits for the open question below.

**How a menu is built.** The app builds every menu, `menus::context(place, target)`, from the place's own groups and the items tools add.

1. The place's own groups come first, as [gui.md](../gui.md#context-menus) lists them.
2. Then one group for each tool that adds items, a separator between groups: the processing tools in pipeline order, then the utilities in `UTILITIES`' order. Nothing about a tool's group is chosen by the tool but its items.
3. In the photo's and a thumbnail's menus, an item whose action cannot run on the target is hidden, as macOS's context menus hide what does not apply, whatever stops it: Discard History… where the platform has no alert to ask with is hidden there, though drawn disabled with its tip in Edit and History's ⋯. One that waits only for the photo to become live, as a photo just made active does, is shown disabled, since it runs in a moment. ⋯ is a pull-down from a button, not a context menu, and keeps the pull-down's rule: an item that cannot run is drawn disabled with its tip, as Move to Tab › draws the tab a section is in (83).
4. A submenu goes one level deep. The photo's and a thumbnail's menus show no keys, as macOS's context menus show none.
5. A test holds the photo's menu and a thumbnail's, with every item of every tool shown, to at most 16 items. An item that would pass that needs a decision.
6. An item added to the photo's or a thumbnail's menu is something people see, so its tool's page lists it.

**What an item acts on**, `Target`:

- On the photo: the active photo, with the point under the pointer in the photo's coordinates as pickers take them (79), or no point when the menu opens from the keys. An action that needs a point is hidden without one.
- On a thumbnail: its photo. Opening its menu, by a right-click or from the keys on its focused cell, selects it alone and makes it active first, unless it is in the selection ([0037](0037-one-view-the-photos-tab.md)), so the menu never acts on a photo other than the one it was opened on, and in Phase 2, where the selection is the active photo alone, every item acts on the active photo as it would from the menu bar. From Phase 3, the selection when the photo is in it; while several photos are selected, an item that acts on one photo alone is hidden.
- In ⋯: its tool.

**Opening.**

- A right-click opens a menu, and so does a Control-click on the Mac. On a focused element, Shift+F10, the Menu key and AccessKit's ShowContextMenu open it too (`components::context_menu`).
- It opens at the pointer, or under the element from the keys: native on the Mac, drawn elsewhere (40). A long press on touch and pen comes later.
- While a picker is on, the photo's right-click is the picker's (79), and later a mask's while one is being drawn; the photo's menu opens only when nothing on the photo takes the right-click.
- The photo is no focus stop in Phase 2, so its menu has no key. Every item in it is in the menu bar too.

**Every item is reachable without a right-click:** in the menu bar, in its tool's section or ⋯, or by its keys. A right-click menu is a shortcut and never the only way. A tool's action has its row or button by construction, and a test holds every utility's item to a menu bar item.

**The places' items now:**

| Place | The app's groups | Tools' groups |
| --- | --- | --- |
| The photo | Copy Edit, Apply Edit, Reset Edit; Export…; Background ▸; Show in Finder joining once folder mode can reveal a photo | History: Discard History… |
| A thumbnail | Copy Edit, Apply Edit, Reset Edit; Export…; Show in Folder once built | History: Discard History…, hidden while several photos are selected, until discarding several is decided |
| ⋯ | Reset, then Copy Settings and Paste Settings with Copy and Apply Parts (Phase 3), first; the layout's items ([0083](0083-the-side-panels-layout-dragged-and-floating.md)) last | The tool's own, between them; History's are Compress History (Truncate History its alternate), Create a Style… and Discard History… |

**The first slice** (planned) builds:

- `Place`, `MenuItem`, `Definition::menus`, `Utility::menus` and their checks;
- the generated `tool-<tool>-<action>` actions;
- `Target` and `menus::context(place, target)`;
- the photo's menu and a thumbnail's;
- History's Discard History… in both, the first item a tool adds.

Pickers as items, the other places and ⋯'s Reset, Copy Settings and Paste Settings come later.

## Options considered

- **A tool builds its menu with a callback.** That needs egui or app types in `apertrust-tools` (28). It cannot be shown as the Mac's native menu, which runs nothing (40). It gives its items no keys, no place in the menu bar and nothing a script sees.
- **The app hard-codes every place's items, and tools add none.** The simplest, but a tool's items would live in `menus.rs`, away from the tool.
- **Only the focused tool adds items to the photo's menu, or tools' items go under a Tools ▸ submenu.** Shorter menus, but an item moves with the focus or hides a level down. The limit per tool and the test of the whole menu keep it short instead.
- **A context menu on a tool's header or its controls.** The header already has its presets button and ⋯, and a slider's right-click is its fine control (42), so a menu there would give one gesture two meanings on a tool.
- **Each tool choosing its group's place or an order number.** Tools would compete for the top. Pipeline order is stable and the same for everyone.

## Consequences

- `apertrust-tools`: `Place`, `MenuItem`, `Definition::menus`, `Utility::menus`, and the `DefinitionError`s for an item that names no action, one repeated and one past the limit.
- The app:
  - the generated tool actions in `actions.rs`;
  - `Target`;
  - `menus::context(place, target)` in place of the surround's form;
  - the photo's right-click in `main.rs`, and a thumbnail's from the grid panel;
  - tests of the limits, of the menu's size and of every item being reachable elsewhere.
- `apertrust-ui`: the grid panel reports a cell's right-click and its menu keys, with the cell. It still holds no tool types.
- The engine and the session protocol are unchanged: an item runs an action that already sends its command.
- Decision 78's Discard History… is in the photo's and a thumbnail's menus too. Decisions 83 and 86 no longer open anything on a right-click on a section's header.
- Open: an item that picks at the pointer, such as "Set Exposure from Here", needs a pick at one point that starts, reads and ends as one completed change, which the picker commands (79) may not express. It is settled when a tool first adds one.
