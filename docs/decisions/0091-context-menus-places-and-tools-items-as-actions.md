# 0091. Context menus: places, and tools' items as actions

## Context

A right-click menu helps in a few common places: the photo, a thumbnail, and later a History step, a layer or a job's row. Tools want to add items there, History's Reset first, and whatever belongs to a tool should stay in that tool's code. Three things constrain how:

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
- A utility's items are the registry actions it already has, as History's Reset is `reset`.

**Tools declare their items as data.**

- A tool: `Definition::menus: &'static [MenuItem]`, where `MenuItem { place: Place, section: Section, order: u32, entry: MenuEntry }` names one of the tool's own actions, `MenuEntry::Action(id)`.
- A utility, which has no definition: `Utility::menus` in `utility::UTILITIES`, each item naming the registry action that runs it.
- The app declares its own items for each place the same way, as `MenuItem`s, so its items and the tools' are placed by one rule.
- **A submenu** is the app's alone: `MenuEntry::Submenu(id)` names a submenu of the menu bar by its title's id, as Background ▸ names View › Background (`menu-background`). The builder reads that submenu's items, in its order, as the item's children, so the two never differ and every child is in the menu bar by construction. Its children follow the item rules below, and the submenu is hidden when none of them is shown. `Definition::check` refuses a submenu from a tool, and an app test refuses one that names no submenu of the menu bar.
- **`section`** says which section of the menu an item is in. `Section` is a set of named constants, in `apertrust-tools` and shared by the app and the tools, whose value sets the section's place in a menu, 1000 apart: `Section::EDIT` 1000, `EXPORT` 2000, `VIEW` 3000, `HISTORY` 4000, and `LAYOUT` 9000 for ⋯'s layout items, last. A tool may use a section value of its own between 1 and 8999; `LAYOUT` and above are the app's, so the layout's items stay last. `Definition::check` refuses a section of 9000 or more, and an app test holds the utilities to the same range.
- **`order`** places an item within its section, 100 apart (100, 200, 300 and on), lower first.
- The gaps leave room to put an item between two others without renumbering them.
- A tool with nothing to add declares nothing.
- `Definition::check` refuses an item that names none of the tool's actions, an item listed twice, and more than two items in one place among `Photo` and `Thumbnail`, or more than four in `Options`. An app test holds the utilities' items to the same limits and to actions the registry declares.
- A picker is no item yet: an item that picks at the pointer waits for the open question below.

**How a menu is built.** The app builds every menu, `menus::context(place, target)`, from the app's items for the place and the items tools add.

1. Every item for the place, the app's and those tools add, is sorted by `(section, order)`.
2. A separator falls wherever `section` changes. A section with no item shown, empty or all hidden, leaves no line.
3. A test fails on two items with the same `(section, order)` in one menu, whoever declares them. It builds each menu as it would show: the photo's and a thumbnail's, with every tool's and utility's items for the place, and each tool's and utility's ⋯, the app's items with that one's own. Two tools may use the same `(section, order)` in their own ⋯, which never show together.
4. In the photo's and a thumbnail's menus, an item whose action cannot run on the target is hidden, as macOS's context menus hide what does not apply, whatever stops it. One that waits only for the photo to become live, as a photo just made active does, is shown disabled, since it runs in a moment. ⋯ is a pull-down from a button, not a context menu, and keeps the pull-down's rule: an item that cannot run is drawn disabled with its tip, as Move to Tab › draws the tab a section is in (83).
5. A submenu goes one level deep. The photo's and a thumbnail's menus show no keys, as macOS's context menus show none.
6. A test holds the photo's menu and a thumbnail's, with every item of every tool shown, to at most 16 items. An item that would pass that needs a decision.
7. An item added to the photo's or a thumbnail's menu is something people see, so its tool's page lists it.

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

| Place | The app's items | Tools' items | The menu reads |
| --- | --- | --- | --- |
| The photo | Export… (`EXPORT`, 100); Background ▸ (`VIEW`, 100); Show in Finder joining once folder mode can reveal a photo | History's Reset (`EDIT`, 100) | Reset, —, Export…, —, Background ▸ |
| A thumbnail | Export… (`EXPORT`, 100); Show in Folder once built | History's Reset (`EDIT`, 100), hidden while several photos are selected, since it acts on one photo | Reset, —, Export… |
| ⋯ | Reset, then Copy Settings and Paste Settings with Copy and Apply Parts (Phase 3), in `EDIT`; the layout's items ([0083](0083-the-side-panels-layout-dragged-and-floating.md)) in `LAYOUT` | The tool's own, in a section of its choosing between them; History's are Compress History (Truncate History its alternate), Create a Style… and Discard History… | — |

Discard History… is in no context menu: it stays in Edit and History's ⋯ ([0078](0078-undo-redo-and-the-edits-history.md)).

**The first slice** (planned) builds:

- `Place`, `Section`, `MenuItem`, `MenuEntry`, `Definition::menus`, `Utility::menus` and their checks;
- the generated `tool-<tool>-<action>` actions;
- `Target` and `menus::context(place, target)`;
- the photo's menu and a thumbnail's, with Background ▸ read from View › Background (`MenuEntry::Submenu`);
- History's Reset in both, the first item a tool adds.

Pickers as items, the other places and ⋯'s Reset, Copy Settings and Paste Settings come later.

## Options considered

- **A tool builds its menu with a callback.** That needs egui or app types in `apertrust-tools` (28). It cannot be shown as the Mac's native menu, which runs nothing (40). It gives its items no keys, no place in the menu bar and nothing a script sees.
- **The app hard-codes every place's items, and tools add none.** The simplest, but a tool's items would live in `menus.rs`, away from the tool.
- **Only the focused tool adds items to the photo's menu, or tools' items go under a Tools ▸ submenu.** Shorter menus, but an item moves with the focus or hides a level down. The limit per tool and the test of the whole menu keep it short instead.
- **A context menu on a tool's header or its controls.** The header already has its presets button and ⋯, and a slider's right-click is its fine control (42), so a menu there would give one gesture two meanings on a tool.
- **The app's groups first, then one group per tool in pipeline order, then the utilities.** A tool's item could never sit beside the app's items it belongs with, as History's Reset beside Export… and the edit's other commands.
- **A submenu's children declared in the context menu, or by a callback naming a source.** Either would hold a second list of View › Background's choices, or code in a declaration; reading the menu bar's submenu keeps one list.
- **Tools' sections unbounded, or a sort that puts `LAYOUT` last whatever the values.** The first breaks "layout last"; the second hides a rule in the sort. A range below `LAYOUT` says it in the check.
- **A single weight number for each item, with bands of it for the separators.** One number would do, but the rule for where a separator falls would be hidden in its value. Two fields say it plainly: `section` for the line, `order` within it.
- **The words.** "Order", lower first, rather than "weight" or "priority", whose direction is ambiguous: a heavier item could sink or rise, and a higher priority could come first or last.

## Consequences

- `apertrust-tools`: `Place`, `Section`, `MenuItem`, `MenuEntry`, `Definition::menus`, `Utility::menus`, and the `DefinitionError`s for an item that names no action, one repeated, one past the limit, a section of `LAYOUT` or above, and a submenu from a tool.
- The app:
  - the generated tool actions in `actions.rs`;
  - `Target`;
  - `menus::context(place, target)` in place of the surround's form;
  - the photo's right-click in `main.rs`, and a thumbnail's from the grid panel;
  - the app's own items as `MenuItem`s;
  - the submenus the app's items name, read from the menu bar's (`menus::BACKGROUND` first);
  - tests of the limits, of the menu's size, of a repeated `(section, order)` in one menu, of a submenu naming one of the menu bar's, and of every item being reachable elsewhere.
- `apertrust-ui`: the grid panel reports a cell's right-click and its menu keys, with the cell. It still holds no tool types.
- The engine and the session protocol are unchanged: an item runs an action that already sends its command.
- Decisions 83 and 86 no longer open anything on a right-click on a section's header.
- Open: an item that picks at the pointer, such as "Set Exposure from Here", needs a pick at one point that starts, reads and ends as one completed change, which the picker commands (79) may not express. It is settled when a tool first adds one.
