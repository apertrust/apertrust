# 0091. Context menus: places, and tools' items as actions

## Context

A right-click menu helps in a few common places: the photo, a thumbnail, and later a History step, a layer or a job's row. Tools want to add items there, History's Reset first, and whatever belongs to a tool should stay in that tool's code. Three things constrain how:

- `apertrust-tools` has no egui and no app types ([0028](0028-tools-bound-to-operations.md)).
- The Mac's native menus run nothing and answer with the id chosen ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)).
- Every command a person can give is an action of the registry, so that its keys, its menu items and its buttons agree ([0036](0036-the-design-system.md)).

A tool's section already has its own buttons: reset, presets and options (⋯). A slider's right-click is darktable's fine control ([0042](0042-numbers-shown-typed-and-stored.md)).

## Decision

**Context menus open only in named places, and tools add items to them as data.** Every item is a registry action, the app builds every menu, and no tool runs code in a menu.

**The places.** `Place`, in `apertrust-tools`, names the places open to tools' items and no other: `Photo`, `Thumbnail` and `Options`. A tool's or a utility's item can name no other place, since its type has none. The app's own list is `menus::Place`: `Open(apertrust_tools::Place)` for those three, and one variant for each place of the app's alone, added with its place's menu, `Surround` first.

- Open to tools' items: `Photo` (the photo in the central panel), `Thumbnail` (a grid panel cell, and a cell in Photo when several photos show), and `Options` (a tool's own ⋯, in its own section only).
- The app's alone, each built with its feature:
  - the surround, `Place::Surround` (built: View › Background's items);
  - the grid bar (built: Position ▸);
  - Browse's folder rows (built);
  - the grid panel's empty area: the grid bar's options, with Select All and Select None;
  - a History step: Go to Step, Show Tool, Compress up to Here and Truncate after Here;
  - the jobs window's rows: Cancel, Show in Folder and Copy Error;
  - a layer (Phase 4), the scopes, a tab's icon.
- Another place opens to tools' items only by a decision of its own, which adds it to `apertrust-tools`' `Place`.
- The top panel, `TopPanel`, is the app's alone: a right-click anywhere on it, an item or its empty areas, opens the toolbar menu ([0092](0092-the-toolbar.md)), the system's window menu staying on the system's own key ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)).

**A tool takes no right-click.**

- Its header, its body and its controls open no context menu.
- Its presets come from the header's presets button.
- Its own menu is the header's options button, ⋯, a pull-down from that button.
- Shift+F10 and the Menu key on a focused header open nothing; Tab reaches ⋯.
- A control keeps its own gesture, which is never a menu: a slider's right-click opens darktable's fine control (42), and a picker button's takes an area ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)).
- The rows of a list a tool shows are items of their own and may have a menu, as a place does: Browse's folders, the Keywords list ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)), History's steps and layers. The Colour picker's stored samples keep darktable's right-click load and store (79).

**Every item is an action** of the registry (36). So it can be given keys, it is in the menu bar or its tool too, it is enabled and checked from the state, and the native menu only reports which was chosen.

- A tool's own actions are registry actions, generated as the app starts from every tool's definition, one for each `Row::Action` and each button of `Row::Buttons` (`actions::tool_actions`):
  - they are made once, into a `OnceLock` beside the static `ACTIONS`, and every lookup by id finds them (`actions::action`, `named`, `all`): the keymap, the keys settings keep, the context menus and the Mac's menus;
  - its id is `tool-<tool>-<action>`, under which settings keep its keys;
  - its words are the action's full name, `tool-<tool>-action-<action>-full`;
  - it has no default keys and works in the view's scope;
  - it is available while the photo takes edits, live and not open for viewing only, on the main window, with no modal dialog over it;
  - a button's runs as pressing the button does: `Intent::ToolAction`, then `Change::Action` on the tool's values the mirror holds, `ProcessingTool::action`, and a `SetControl` for each control it writes, one change and one history step;
  - a `Row::Action`'s, whose row is drawn disabled until what it runs is built, is listed and never runs.
- A utility's items are the registry actions it already has, as History's Reset is `reset`.

**Tools declare their items as data.** Every item, a tool's, a utility's or the app's, is a `MenuItem { place: Place, section: Section, order: u32, entry: MenuEntry }`, so the app's items and the tools' are placed by one rule.

- `MenuEntry::Action(id)` is one action. `MenuEntry::Submenu { title, actions }` is a submenu of actions, one level deep: `title` a message id, `actions` from one to eight (`MOST_IN_SUBMENU`), each once, in order. A submenu counts as one item towards its place's limit.
- A processing tool: `ProcessingDefinition::menus: &'static [MenuItem]`, each action one of the tool's own, a submenu's children included.
- A utility: `Utility::menus` in `utility::UTILITIES`, each action the registry action that runs it.
- Planned, both in one field, `ToolDefinition::menus`, the parent every tool's definition embeds ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)); its items at `Place::Options` are the tool's ⋯, before the layout's, History's Compress History, Truncate History, Create a Style… and Discard History… among them, Truncate History an item of its own.
- The app: `menus::APP_ITEMS`, each action a registry action's id.
- **Background ▸** is the app's `Submenu { title: "menu-background", actions: [View › Background's ids] }`, and an app test keeps its actions equal to the menu bar's `menus::BACKGROUND`, in its order.
- A submenu's children follow the item rules below: each hidden or shown disabled as an item is, and none shows its keys. A submenu left with no child shown is hidden.
- **`section`** says which section of the menu an item is in. `Section` is a set of named constants, in `apertrust-tools` and shared by the app and the tools, whose value sets the section's place in a menu, 1000 apart: `Section::EDIT` 1000, `EXPORT` 2000, `VIEW` 3000, `HISTORY` 4000, and `LAYOUT` 9000 for ⋯'s layout items, last.
  - A tool's or a utility's item takes a section from `EDIT` up to, not including, `LAYOUT`, a named one or a value of its own: below `EDIT` and from `LAYOUT` up are the app's, so its items fall between the app's editing items and the layout's, which stay last.
  - In `EDIT`, order 100 is Reset's, the app's in ⋯ and History's on the photo and a thumbnail, so a tool's or a utility's item there takes order 200 or more and Reset stays first.
  - `check_section` holds a tool's items to both rules, refusing the section (`DefinitionError::MenuSection`) or the order (`MenuOrder`); an app test holds the utilities to the same, History's Reset alone at `(EDIT, 100)`. The app's own items are exempt.
- **`order`** places an item within its section, 100 apart (100, 200, 300 and on), lower first.
- The gaps leave room to put an item between two others without renumbering them.
- A tool with nothing to add declares nothing.
- `ProcessingDefinition::check` refuses an item that names none of the tool's actions (`UnknownMenuAction`), an action listed twice in one place, a submenu's children included (`DuplicateMenuItem`), a submenu of no action or of more than eight (`SubmenuItems`), and more than two items in one place among `Photo` and `Thumbnail`, or more than four in `Options` (`MenuItems`), through `check_menus`, and the section rules through `check_section`. An app test holds the utilities' items to the same limits and to actions the registry declares, each in the menu bar on every platform.
- A picker is no item yet: an item that picks at the pointer waits for the open question below.

**How a menu is built.** The app builds every menu, `menus::context(place, target)`, from the app's items for the place and the items tools and utilities add. The surround's is View › Background's items alone, read from `menus::BACKGROUND`.

1. Every item for the place, the app's and those tools add, is sorted by `(section, order)`.
2. A separator falls wherever `section` changes. A section with no item shown, empty or all hidden, leaves no line.
3. A test fails on two items with the same `(section, order)` in one menu, whoever declares them. It builds each menu as it would show: the photo's and a thumbnail's, with every tool's and utility's items for the place, and each tool's and utility's ⋯, the app's items with that one's own. Two tools may use the same `(section, order)` in their own ⋯, which never show together.
4. In the photo's and a thumbnail's menus, an item whose action cannot run on the target is hidden, as macOS's context menus hide what does not apply, whatever stops it. One that waits only for the photo to become live, as a photo just made active does, is shown disabled, since it runs in a moment. ⋯ is a pull-down from a button, not a context menu, and keeps the pull-down's rule: an item that cannot run is drawn disabled with its tip, as Move to Tab › draws the tab a section is in (83).
5. A submenu goes one level deep. The photo's and a thumbnail's menus show no keys, as macOS's context menus show none.
6. A test holds the photo's menu and a thumbnail's, with every item of every tool shown, to at most 16 items, a submenu's children each counted. An item that would pass that needs a decision.
7. An item added to the photo's or a thumbnail's menu is something people see, so its tool's page lists it.

**What an item acts on**, `Target`:

- On the photo: the active photo, with the point under the pointer in the photo's coordinates as pickers take them (79), or no point when the menu opens from the keys. An action that needs a point is hidden without one.
- On a thumbnail: its photo. Opening its menu, by a right-click or from the keys on its focused cell, selects it alone and makes it active first, unless it is in the selection ([0037](0037-one-view-the-photos-tab.md)), so the selection the menu acts on always includes the photo it was opened on, and in Phase 2, where the selection is the active photo alone, every item acts on the active photo as it would from the menu bar. From Phase 3, the selection when the photo is in it; while several photos are selected, an item that acts on one photo alone is hidden.
- In ⋯: its tool.
- On the surround: the app.

**Opening.**

- A right-click opens a menu, and so does a Control-click on the Mac. On a focused element, Shift+F10, the Menu key and AccessKit's ShowContextMenu open it too (`components::context_menu`).
- It opens at the pointer, or under the element from the keys: native on the Mac, drawn elsewhere (40). A long press on touch and pen opens it in Touch mode, planned ([0094](0094-one-interface-standard-and-touch-modes.md)).
- While a picker is on, the photo's right-click is the picker's (79), and later a mask's while one is being drawn; the photo's menu opens only when nothing on the photo takes the right-click.
- The photo is no focus stop in Phase 2, so its menu has no key. Every item in it is in the menu bar too.

**While a menu is open** (`context_popup`):

- **A drawn menu opened while its photo waits** to go live, as a thumbnail's does on the photo it has just made active, shows the items that wait disabled. Once the photo is live it is rebuilt in place, at the same anchor and with the same opener, so they are enabled without the menu reopening.
- **The photo's and a thumbnail's act on the photo asked for as they opened.** When another photo becomes the one asked for while one is open, by a drop, Open… or an open asked for earlier, the menu is dismissed, taking no answer, so it never acts on a photo other than its own.
- **The Mac's native menu is modal:** nothing changes while it is open, and it keeps what it opened with until it closes.

**Every item is reachable without a right-click:** in the menu bar, in its tool's section or ⋯, or by its keys. A right-click menu is a shortcut and never the only way. A tool's action has its row or button by construction, and a test holds every utility's item to a menu bar item.

**The places' items now:**

| Place | The app's items | Tools' items | The menu reads |
| --- | --- | --- | --- |
| The photo | Export… (`EXPORT`, 100); Background ▸ (`VIEW`, 100); Show in Finder joining once folder mode can reveal a photo | History's Reset (`EDIT`, 100); Orientation's Orientation ▸ (`VIEW`, 200), its four buttons: rotate 90° anticlockwise, rotate 90° clockwise, flip horizontally and flip vertically | Reset, —, Export…, —, Background ▸, Orientation ▸ |
| A thumbnail | Export… (`EXPORT`, 100); Show in Folder once built | History's Reset (`EDIT`, 100), hidden while several photos are selected, since it acts on one photo | Reset, —, Export… |
| ⋯ | Reset, then Copy Settings and Paste Settings with Copy and Apply Parts (Phase 3), in `EDIT`; the layout's items ([0083](0083-the-side-panels-layout-dragged-and-floating.md)) in `LAYOUT` | The tool's own, in a section of its choosing between them; History's are Compress History, with Truncate History its alternate while Option is held, Create a Style… and Discard History…; planned, Truncate History an item of its own ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)) | — |

Discard History… is in no context menu: it stays in Edit and History's ⋯ ([0078](0078-undo-redo-and-the-edits-history.md)).

**The first slice** is built:

- in `apertrust-tools`: `Place` (`Photo`, `Thumbnail`, `Options`), `Section`, `MenuItem`, `MenuEntry` with its submenus, `MOST_IN_SUBMENU`, `ToolDefinition::menus`, `Utility::menus`, `check_menus` and `check_section` with their `DefinitionError`s; `UTILITIES` in a `utility` module, moved from `about`;
- in the app:
  - the generated `tool-<tool>-<action>` actions (`actions::tool_actions`), run through `Intent::ToolAction` and `tool_action`;
  - `menus::Place` with `Surround`, the surround's menu built by `menus::context` too;
  - `Target`, `menus::APP_ITEMS` and `menus::context(place, target)`;
  - the photo's right-click menu (`central_menu`), a picker on taking the right-click first, and a thumbnail's from the grid panel's cell (`CellMenu`), both through `context_popup`, with its two behaviours while a menu is open;
- the items: History's Reset in both menus, and Orientation ▸ on the photo, after Background ▸, the first submenu a tool adds;
- the tests: the limits and the section rules, the menus' size, a repeated `(section, order)` in one menu, Background ▸ equal to View › Background, every item reachable elsewhere, the photo's and a thumbnail's menus as they read, and the generated actions' ids, words, availability and keys.

Pickers as items, the other places, and ⋯ built through `menus::context` with Reset, Copy Settings and Paste Settings come later.

## Options considered

- **A tool builds its menu with a callback.** That needs egui or app types in `apertrust-tools` (28). It cannot be shown as the Mac's native menu, which runs nothing (40). It gives its items no keys, no place in the menu bar and nothing a script sees.
- **The app hard-codes every place's items, and tools add none.** The simplest, but a tool's items would live in `menus.rs`, away from the tool.
- **Only the focused tool adds items to the photo's menu, or tools' items go under a Tools ▸ submenu.** Shorter menus, but an item moves with the focus or hides a level down. The limit per tool and the test of the whole menu keep it short instead.
- **A context menu on a tool's header or its controls.** The header already has its presets button and ⋯, and a slider's right-click is its fine control (42), so a menu there would give one gesture two meanings on a tool.
- **The app's groups first, then one group per tool in pipeline order, then the utilities.** A tool's item could never sit beside the app's items it belongs with, as History's Reset beside Export… and the edit's other commands.
- **Submenus the app's alone.** A tool with a row of related buttons, as Orientation's four, would need an item each, past its limit of two, or leave some out; a submenu of its own actions keeps them together as one item.
- **A submenu naming a menu-bar submenu by its title's id,** its children read from it. One list for Background ▸, but no form a tool could use, since its actions are in no menu-bar submenu. Listing the children in the item, as a tool's are, with a test holding Background ▸ to View › Background, gives every submenu one form.
- **Every place in `apertrust-tools`' `Place`, a check refusing a tool's item in the app's.** A test would catch what the type rules out as it compiles, and the tools' crate would name places it never uses.
- **Tools' sections unbounded, or a sort that puts `LAYOUT` last whatever the values.** The first breaks "layout last"; the second hides a rule in the sort. A range below `LAYOUT` says it in the check.
- **A single weight number for each item, with bands of it for the separators.** One number would do, but the rule for where a separator falls would be hidden in its value. Two fields say it plainly: `section` for the line, `order` within it.
- **The words.** "Order", lower first, rather than "weight" or "priority", whose direction is ambiguous: a heavier item could sink or rise, and a higher priority could come first or last.

## Consequences

- `apertrust-tools`: `Place`, `Section`, `MenuItem`, `MenuEntry`, `MOST_IN_SUBMENU`, `ToolDefinition::menus`, `Utility::menus`, `check_menus`, `check_section`, and the `DefinitionError`s for an item that names no action, one repeated, a submenu of none or too many, too many in a place, a section outside `EDIT` up to `LAYOUT` and an `EDIT` order under 200; `UTILITIES` in a `utility` module.
- The app:
  - the generated tool actions in `actions.rs`, found by every lookup by id;
  - `menus::Place` and `Target`;
  - `menus::context(place, target)` for every context menu, the surround's included;
  - the photo's right-click in `main.rs`, and a thumbnail's from the grid panel;
  - the app's own items as `MenuItem`s, `menus::APP_ITEMS`;
  - tests of the limits, of the menu's size, of a repeated `(section, order)` in one menu, of Background ▸ against View › Background, and of every item being reachable elsewhere.
- `apertrust-ui`: the grid panel reports a cell's right-click and its menu keys, with the cell (`GridPanelOutput::menu`). It still holds no tool types.
- The engine and the session protocol are unchanged: an item runs an action that already sends its command.
- Decision 92's tools' items on the toolbar are these generated actions.
- Decisions 83 and 86 no longer open anything on a right-click on a section's header.
- Open:
  - An item that picks at the pointer, such as "Set Exposure from Here", needs a pick at one point that starts, reads and ends as one completed change, which the picker commands (79) may not express. It is settled when a tool first adds one.
  - A processing tool's generated action is available while the photo takes edits (a utility's items name registry actions, each with its own availability, [0093](0093-one-tool-model-processing-tools-and-utility-tools.md)); it does not ask whether its own control can take a change on that photo, as its row in the panel may. Every built tool's buttons apply to every photo, so nothing differs yet. It is settled when a tool's action first cannot run on a photo that takes edits, by an availability the tool declares or the app derives from its definition.
