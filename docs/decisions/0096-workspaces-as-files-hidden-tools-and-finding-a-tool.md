# 0096. Workspaces as files, hidden tools and finding a tool

## Context

[0038](0038-the-side-panels-tabs-by-editing-stage.md) gives the Default workspace fixed tabs, and [0083](0083-the-side-panels-layout-dragged-and-floating.md) lets a person reorder, pin and move a tool on screen, display order only. Four things that layout needs are settled here:

- **Which tabs a workspace has.** Tabs were the build's: a person could not add one, name it or give it an icon outside a Customise tabs dialog not yet built.
- **A tool a person never uses.** Every tool in a tab of the Default workspace leaves no way to clear it of the tools a person never uses.
- **Finding a tool.** With more than 60 tools once the migration ends, a person has to know which tab holds one; darktable's module search was left for later (38).
- **Where workspaces are kept.** A workspace is a set of lists a person may want to copy, share or write by hand, and settings keys are permanent unless migrated ([0095](0095-the-windows-parts-named-by-their-role.md)).

darktable at the reference release: its module group presets (`src/libs/modulegroups.c`) each hold groups with a name, an icon and a list of modules, whether the search box shows, and a quick access panel; they are kept in `data.db` as one string per preset, a whole copy that a new release's modules never join, which reach a person's preset only through search or the active-modules group. Its search box (`_lib_modulegroups_update_iop_visibility`) matches a module's name, its aliases ("search modules by name or tag", `dt_iop_get_localized_aliases`) and an instance's name, case-folded, across every module whatever the group, and deselects the group's button while it has text; it has no default key.

A workspace is interface state, never the edit's ([0037](0037-one-view-the-photos-tab.md), 83); every item a menu holds is reachable without a right-click ([0091](0091-context-menus-places-and-tools-items-as-actions.md)); Export stays in every workspace ([0090](0090-export-as-a-photos-tab-tool.md)).

## Decision

**A workspace is its tabs, each tab's tools, the tools it hides and its toolbar, kept in a file of its own.** Default and Essentials are built in, each a file in the repository's `data/workspaces/`; a person adds more by writing a file. A tool may be in no tab of a workspace: it is hidden, and still reachable from the Hidden tab and by Find Tool…. This is darktable's module group presets in Apertrust's shell. Built: Phase 2's steps 1 and 2; planned: steps 3 to 7 and Phase 3's, below.

### What a workspace holds

| Part | Holds |
| --- | --- |
| Its tabs | In order, each an id, a title and an icon |
| Each tab's tools | Its pinned tools, top first, then the rest, top first (83) |
| The tools it hides | Tools in no tab of it, shown only in the Hidden tab |
| Its toolbar | The Toolbar's items in order ([0092](0092-the-toolbar.md), Phase 3) |
| The tools that float | Each with the tab it left (83, Phase 3) |

Not the Tools panel's width, a floating panel's place, a section's open state, the theme or the keys: they suit a screen or a person, not a task, and stay app-wide in `settings.toml`. Never a photo or an edit.

### Built in

- **Default,** id `default`, `data/workspaces/default.toml`, the one shown where settings name none: 38's tabs and lists, Photos, Adjust, Detail, Geometry, Effects, Raw and History, Scopes pinned in the five editing tabs. It opens on Adjust.
- **Essentials,** id `essentials`, `data/workspaces/essentials.toml`, for the tools most edits need: Photos (Collections, Export); Adjust, Scopes pinned over White balance, Exposure, AgX, Tone equalizer, Colour balance RGB and Orientation, in that order; History. Every other tool is in no tab, so hidden: a workspace decides only what is shown, and a hidden tool is still processed. It opens on Adjust.
- **No darktable workspace:** a person who wants darktable's module groups writes them as a workspace of their own (38).
- **Photos,** for culling, in Phase 3 ([gui.md](../gui.md#workspaces)); **Map** with the map.
- **A built-in workspace is a data file,** `data/workspaces/<id>.toml`, in the format below, embedded at build time by the app (decision 45), read by the same reader as a person's own (`Layout::defaults`, plain data with no egui) and never from an installed folder. It is the one source of the workspace: nothing of its tabs or tools is in code. It lists no `seen`, its lists being its defaults. In the user data folder it has no file until a person first changes it, when its file is written whole; **Reset Workspace** deletes the file, so the one shipped applies again. A built-in workspace nobody changed follows each build's defaults; once its file exists, a later build's new tools still reach it (below), but its reordered defaults do not until Reset Workspace.

### The workspace's file

**One file per workspace, `<user data>/workspaces/<id>.toml`,** one format for a built-in's and a person's own; the id is the file's name, lower-case letters, digits and hyphens. `default.toml` and `essentials.toml` are the built-ins' changed copies; any other is a person's own.

```toml
version = 1
name = "Portraits"                      # a person's own; a built-in's name is Fluent's
hidden = ["vignette", "grain"]
seen = ["collections", "export", "scopes", "history", "temperature", "exposure", "…"]
toolbar = ["mark", "import", "export", "undo", "redo"]   # decision 92, Phase 3

[[tabs]]
id = "photos"

[[tabs]]
id = "skin"
title = "Skin"
icon = "adjust"

[[tabs]]
id = "history"

[pinned]
skin = ["scopes"]

[order]
photos = ["collections", "export"]
skin = ["temperature", "exposure", "colorbalancergb"]
history = ["history"]

[floating]                              # decision 83, Phase 3
scopes = "skin"
```

- **`version`,** the file format's own, independent of `settings.toml`'s, migrated as `settings.toml` is: a newer version's file is read for what this build knows and never overwritten; one that is not TOML is set aside as `<id>.toml.bak`, with a warning, and the built-in's defaults apply, or a person's own is not listed.
- **`name`:** a person's own workspace's name, as written, the same in every language; its id where it has none. A built-in's name is its message, `workspace-default` and `workspace-essentials`, translated: neither its bundled file nor a changed copy writes a `name`, and one written there is never shown.
- **`tabs`,** an array of tables in order, each `id`, `title` and `icon`:
  - a tab id a build knows, a tab of a built-in workspace's (Default's and Essentials', `photos`, `history` among them), takes its title from Fluent by its id (`tab-<id>`) and its icon from that built-in's file, so its words follow the language; a `title` or `icon` written for it overrides them. A built-in's file writes the id and the icon, never a title.
  - A person's own tab has the `title` written, the same in every language, and its icon by name, one of `apertrust-ui`'s icons as a toolbar item's is (92); with no title its id shows, and with no icon or an unknown one Lucide's `square`.
  - `hidden` is reserved, the Hidden tab's id. At most 12 of the file's tabs show, `panel-tab-1` to `panel-tab-12` choosing them by position; a tab past them is not shown, with a warning, and stays in the file. Such a tab counts as absent: the tools only it holds are in no tab, so they show in the Hidden tab and Find Tool… reaches them, without being written into `hidden`, and a new tool is never placed there. The Hidden tab is not one of them: shown, it comes after them, a thirteenth at most, chosen by its own `panel-tab-hidden`. A workspace whose file lists no tab is refused, with a warning: a built-in's defaults apply, or a person's own is not listed.
- **`[pinned]` and `[order]`,** keyed by tab id, each a list of tool ids, top first: a tab's pinned tools and the rest (83). A list for a tab id `tabs` lacks is skipped and kept.
- **`hidden`,** the tools the workspace hides; **`seen`,** every tool the build that wrote the file knew, so a new build's new tool is told from one hidden.
- **Written whole** (from step 4), through the safe write, on a thread of its own, once the layout has stood still for a second after any change, a drop included, and at once when another workspace is chosen and on exit, as settings are. The file is read as the app starts; a write first reads it again and, where it changed on disk since, applies the person's change to it as it is then, so a hand edit made meanwhile is kept. A tool id or tab id this build does not know keeps its place. Opening and closing the app writes nothing. A file a person adds by hand while the app runs is read at the next start; a workspace the visual editor makes (Phase 3) is registered at once, its `workspace-<id>` made then, so it can be chosen without a restart.

### The layout shown

Worked out when the workspace is read and after each change, never per frame (83):

1. Each tab takes its `pinned` and `order` lists; an id in both stays pinned, one twice in a list keeps its first place, an id the build does not know or a tool of the Import sheet's ([0086](0086-import-as-a-full-window-sheet.md)) is skipped and kept.
2. A tool `hidden` names is in no tab, whatever the lists say, but Export; where a hand-edited file also lists it in `[floating]`, `hidden` wins and the floating record is dropped at the next write.
3. **A new tool,** one of the Tools panel's that neither `seen`, `hidden` nor any tab's lists name, goes into each tab shown whose id is one of its default tabs, if the workspace has one, placed after its nearest preceding neighbour in that tab's default list (83's rule); else it is hidden. Its default tabs are the workspace's own defaults for a built-in, and for a person's own Default's, so a tool new in Adjust joins any workspace with a tab `adjust`. Scopes, new in several tabs, goes into each.
4. **A tool in no tab** after these steps, that `seen` names, is hidden as if `hidden` named it, and written there at the next write: a hand edit that drops a tool from every list hides it. A floating section is never hidden this way: it stays floating, and docks at its default place where the tab it left no longer holds it (83). A tool only a tab past the twelfth holds is shown in the Hidden tab but not written into `hidden`.
5. **Export** is never hidden: `hidden` naming it is ignored, and in no tab shown, none or only one past the twelfth, it goes to the end of the Photos tab's scrolling list where the Photos tab is among the tabs shown, else of the first tab shown that holds tools, else of the first tab, as the other fallbacks go, so Ctrl+E always reaches it.

With no file, a built-in workspace is its defaults. A person's own needs only `tabs` and its lists: a tool it names nowhere, in `seen`, `hidden` or a tab's lists, is new and goes by step 3.

### Invariants

Held by the operations and unit-tested: a tool at most once in a tab, `pinned` and `order` never sharing an id; every tool of the Tools panel in a tab of the workspace or hidden in it, never both; Export in a tab of every workspace; no import tool in any tab, in the Hidden tab or in search; a section drawn in one place a frame. 83's invariant that every tool is in a tab of the Default workspace is gone.

### Hidden tools and the Hidden tab

- **The Hidden tab** is computed, not kept: the workspace's hidden tools, processing tools in pipeline order, first at the top, as the Default workspace's Raw lists them, then utilities in its order. Its id is `hidden`, its title Hidden (`tab-hidden`), its icon Lucide's `eye-off` with its filled form. Nothing in it is pinned or reordered.
- **Show Hidden Tools,** a checked item, `tools-panel-show-hidden`, app-wide in `settings.toml` (`tools_panel.show_hidden`, off), shows it last in the tabs and chooses it; unchecked, it goes, and the tab shown before it is shown again. Shown with nothing hidden, it says "No hidden tools in this workspace." It is chosen by a click, by `panel-tab-hidden` and from Tools Panel Tab's list, after the twelve tabs a file may show, so it never pushes one out.
- **A tool is moved out** to a tab by ⋯ › Show in Tab ▸ or a drop on a tab's icon, which takes it out of `hidden`. A drop on the Hidden tab's icon, or ⋯ › Hide, hides a tool.
- **The dot.** While a hidden processing tool changes the photo and the Hidden tab is not shown, a dot, `accent` and 6 points as the Toolbar's Import… carries ([0085](0085-import-in-folder-mode.md)), sits at the top end of the Tools panel's ⋮. A tool changes the photo when it is on, for a tool darktable leaves off (decision 32), or any of its controls is off the value the photo's default steps give it, for one every photo runs; utilities never do. The ⋮ button's tooltip, and Show Hidden Tools' tip in its menu, say "Hidden tools change this photo: Exposure, Vignetting"; screen readers hear it as ⋮'s description, and once, politely, as it appears. Choosing Show Hidden Tools shows the Hidden tab and chooses it.
- **What reveals a tool reveals a hidden one** in the Hidden tab, shown for the moment while Show Hidden Tools is off: View › Collections (L) where Collections is hidden, History's Shift-click on a hidden tool's step. Ctrl+E has Export, never hidden.

### Finding a tool

- **Find Tool… is a capability, not a tool:** a registry action, `find-tool`, on Ctrl+Shift+F (Cmd+Shift+F on a Mac), and a temporary state of the Tools panel. It has no `ToolDefinition` and no section, is never in a tab and is never hidden. It opens from its key, the tab bar's menu and the Tools panel's ⋮.
- **Opening:** the tab bar's row becomes the search field, in the same place and at the tab bar's height, with tab names or without (56 points with them, 36 without), the focus in it, a search icon at its start and ✕ at its end; the tabs are gone while it shows, so their keys and Tools Panel Tab's items do nothing. The tab shown's sections, the pinned ones too, give way to the results.
- **An empty field** shows no list: a hint, centred in `text_secondary`, "Type a tool's name."
- **The results** are the tools' own sections, closed, their headers as everywhere (↺, presets, ⋯). After the title, in `text_secondary`, where it lives: its tab's name, or `eye-off` and Hidden. Opening one gives the real tool, usable there, sending what it sends in a tab; a hidden one's ⋯ holds Show in Tab ▸. Every tool of the Tools panel can be found, hidden ones included, each once, a tool in several tabs (Scopes) named by the first; never an import tool.
- **Matching,** case- and accent-insensitive, and the order, best first:
  1. tools whose name in the interface's language starts with the text;
  2. tools whose name has a word starting with it;
  3. tools matched by their British English name, their darktable module's name or one of darktable's aliases for it, as darktable's search matches them (`dt_iop_get_localized_name`, `dt_iop_get_localized_aliases`), each tool's `.search` attribute in Fluent from darktable's own strings and translations.

  Within each group, processing tools in pipeline order, then utilities. Nothing in a result is highlighted, since a synonym, not the name, may be what matched.
- **Nothing matches:** the hint's place says "No tool matches "…"." with the text typed.
- **Keys:** typing filters at once; ↓ from the field goes to the first result's header, ↑ and ↓ move between the results' headers; Return opens or closes the focused result, and Return in the field opens the first. Esc closes search at once and brings back the tab shown before, with its sections as they were; Esc in an open list or a value field inside a result closes only that. Ctrl+Shift+F again, or ✕, closes it too. Search stays open while a tool found there is used, so a change made there needs no step back.
- **A hidden tool used from search stays hidden;** the dot says whether it changes the photo.
- **A floating tool** (Phase 3) is listed as its header alone, marked Floating; choosing it brings its floating panel to the front, so no section is drawn twice.
- **Screen readers:** the field is a search box named "Find a tool"; after each keystroke a polite live region says how many tools match ("3 tools"); each result's description says where it lives, "in Adjust" or "hidden".
- **Ctrl+Shift+F** is Find Tool's, which nothing else uses. Ctrl+F is kept for View › Search Photos…, planned, searching photos being the more important search, and stays unassigned until it is built.

### The tab bar's menu

- **`Place::TabBar`,** a place of the app's alone (91): a right-click anywhere on the tab bar, on a tab's icon or its empty end, or Shift+F10 and the Menu key on a focused tab, opens it.
- **Its items:** Find Tool…, Show Hidden Tools, Workspace ▸; then Reset Tab, Reset Workspace and Processing Order (83). Opened on a tab's icon, Reset Tab and Processing Order act on that tab, elsewhere on the tab shown; both are disabled on the Hidden tab, and Reset Tab on a tab of a person's own, which has no defaults; Processing Order, which only sorts a tab's processing tools, works there.
- **Workspace ▸** lists every workspace, Default, Essentials, then a person's own by name, the current one checked. Each runs `workspace-<id>`, an action made as the app starts from the workspaces it reads, as tools' actions are (91), with no default keys; settings keep a key by that id.
- **Reset Workspace** deletes the current workspace's file and applies its defaults: a built-in's alone, disabled for a built-in with no file, for a person's own, and for a file a newer build wrote, which this build never overwrites or removes.
- **The Tools panel's ⋮** holds the same items, so none needs a right-click, then its own: Show Tab Names, Customise Tabs…, drawn disabled until the visual editor (Phase 3), and Dock All Floating Panels (Phase 3).

### ⋯ on a tool

The layout's items, last in ⋯ in `LAYOUT` (91), for a tool and a utility alike:

| Item | Does | Disabled when |
| --- | --- | --- |
| Pin, or Unpin | To the end of the tab's pinned list, or the top of its scrolling list | — |
| Move Up, Move Down | One place in its list | First, or last, in it |
| Move to Tab ▸ each tab by name | As a drop on its icon (83) | The tab it is in, one already holding it, a disabled tab |
| Hide | Takes it out of every tab of the workspace into `hidden` | Export, with "Export stays in every workspace"; a floating section, until it docks (83) |
| Show in Tab ▸ each tab by name, for a hidden tool, in place of the others | Moves it there, out of `hidden` | A disabled tab |
| Float, or Dock (planned, Phase 3) | Floats it near the Tools panel's top, or returns it to the place it left (83) | — |
| Position ▸ Top Left, Top Right, Bottom Left, Bottom Right, on a floating panel (planned, Phase 3) | Moves the panel to that corner (83) | — |

- **Keys:** on a focused header Alt+↑ and Alt+↓ (Option on a Mac), 83's. Registry actions with no default keys, each on the tool whose header has the focus: `section-move-up`, `section-move-down`, `section-pin` (Pin or Unpin), `section-move-to-tab-1` to `-12`, `section-hide` and `section-show`, which moves a hidden tool to its first default tab the workspace has, else the first tab that holds tools.
- **Screen readers:** the header gains the custom actions Move up, Move down, Pin or Unpin, and Hide or Show; a polite live region says what happened ("Exposure moved to 3 of 12 in Adjust", "Exposure pinned in Adjust", "Vignetting hidden", "Vignetting shown in Effects"), in every language's messages.
- **Layout moves are not edits:** no step in the history, never undone by Undo (83); none reaches the session.

### Switching

View › Workspace ▸ and the tab bar's menu's Workspace ▸. A switch changes the Tools panel and, from Phase 3, the Toolbar: the Viewer, the selection, the keys and what they act on stay. The tab shown is the one last shown in that workspace; a section's open state is the same in every workspace.

### Where the rest is kept

- **`settings.toml`** keeps `workspace`, the current one's id (`default` by default; an id with no workspace opens Default with a warning, kept until another is chosen), `tools_panel.show_hidden`, and as now the sections' open state and the collapsible groups, `[edit.sections]` and `[edit.groups]`, the same in every workspace. 83's `[workspaces.<id>]` tables are not in it: none is built, so nothing migrates.
- **`state.toml`** keeps the tab last shown in each workspace, `[tab]` keyed by workspace id (`default = "adjust"`), beside the recents, since it changes with every tab chosen and is state, not a setting. `settings.toml`'s `edit.tab` stays in the schema, read only where `state.toml` holds no tab for Default, for its move there, and never written, as `library.recent_folders` moved; it stays in the file as an earlier build wrote it. Neither file's version rises.

### When

Phase 2, in this order, each step one pull request usable on its own:

1. **The workspace model** (built): `workspaces/<id>.toml` as above (tabs with id, title and icon; pinned and order per tab; hidden; seen; version), Default built in, its file `data/workspaces/default.toml`, a person's own workspaces defined by writing a file; `settings.toml`'s `workspace`; the tab last shown in `state.toml`. The workspace's file is read, never written: nothing in this step changes a layout.
2. **Switching** (built): View › Workspace, and Essentials built in, `data/workspaces/essentials.toml`. The workspaces are read once as the app starts, each `workspace-<id>` made then; a switch works out the layout from what was read, with no file read. Until step 3, a tool a workspace leaves out of every tab, Essentials' or a person's own, is reached only by switching to a workspace that shows it, Default placing every tool, or by editing the file.
3. **Hidden tools:** a tool in no tab, the Hidden tab with Show Hidden Tools, Show Hidden Tools in the Tools panel's ⋮ so every hidden tool can be reached from this step on, the dot on ⋮ while a hidden tool changes the photo.
4. **⋯'s layout items:** Pin or Unpin, Move Up and Move Down, Move to Tab ▸, Hide, and Show in Tab ▸ for a hidden tool (`section-show`), so a hidden tool comes back before dragging exists; their keys and screen readers' actions (83's); and writing the workspace's file, as below, this the first step that changes a layout.
5. **The tab bar's menu,** on a right-click and in ⋮ beside step 3's Show Hidden Tools: Find Tool…, Show Hidden Tools, Workspace ▸ (the app's own submenu, as Background ▸ is, listing every workspace as View › Workspace does; decision 91's limit of eight is for a tool's declared submenus), Reset Tab, Reset Workspace (deleting the workspace's file), Processing Order; Find Tool… drawn disabled until step 6.
6. **Search:** Find Tool… (Ctrl+Shift+F), the field in the tabs' place, the tabs disabled, the list empty until typing, hidden tools found too, Esc back to the tab.
7. **Dragging a header,** as 83 designs it: reorder, pin by a drop in the pinned area, move to another tab through its icon.

Phase 3:

- **Floating panels,** 83's step 5.
- **A visual editor for workspaces:** adding, naming and removing tabs, hiding and showing tools, a new workspace; it replaces the Customise tabs dialog, which is not built on fixed lists in Phase 2. Save as Workspace…, Rename Workspace… and Delete Workspace… come with it.
- **The Photos workspace** for culling (37; [gui.md](../gui.md#workspaces)).
- **Each workspace's own toolbar** (92), kept in the workspace's file.

## Options considered

**Where workspaces are kept:**

- **In `settings.toml`, under `[workspaces.<id>]`** (83's plan): a person's own workspace whole among the app's settings, hard to copy or share, every new key permanent unless migrated, and the file's version tied to the layout's. A file per workspace is copied, shared and deleted as a file, and versioned alone.
- **As presets,** darktable's module group presets in `data.db`: a whole copy kept as one string, which never gains a later release's modules, so a person's preset loses each new tool to search. A file of its own, with `seen`, places a new tool in its default tab.
- **A built-in's file holding only what changed,** each tab's lists written only once they change, as 83 had: each build's reordered defaults would reach the tabs nobody changed, at the cost of a file that reads as neither the workspace nor its difference, and of `seen` written for every tab shown. Written whole, the file is the workspace; Reset Workspace brings the defaults back.
- **The tab last shown in the workspace's file:** a write at every tab chosen, to a file a person may share. `state.toml` already keeps what changes as the app is used.

**Hidden tools:**

- **Every tool in a tab of the Default workspace** (83): no way to clear it of the tools a person never uses, short of a workspace of their own.
- **Hidden derived from the lists alone,** with no `hidden`: a hand edit that drops a tool by mistake could not be told from one meant, and the file would not say what it hides. Both are kept: `hidden` for what is meant, `seen` for what is new.
- **A new tool always hidden in a changed workspace,** as darktable's presets leave a new module in no group: a new build's tool would vanish from everyone's Default workspace after one change. Placed by its default tab, it shows where the workspace's tabs expect it, and hidden only where the workspace has none.
- **A new tool at the end of the first tab,** where none of its default tabs is in the workspace (83): it clutters a tab the person chose; the Hidden tab and the dot say it is there.
- **The Hidden tab always shown:** a tab for what the person chose not to see; Show Hidden Tools and the dot reach it when it matters.
- **Show Hidden Tools per workspace, or for the run only:** it suits the person, as Tab Names does, and a switch should not make a tab appear or go; kept app-wide.

**Search:**

- **darktable's always-shown search box:** a row of the panel for a task of a moment, beside the tabs it filters.
- **Results filtering the tab shown:** a tool in another tab or hidden is the one most worth finding.
- **The tool shown in its tab on Esc,** or the Hidden tab shown when a hidden tool was used: Esc should give back what the person had; the dot says whether a hidden tool changed the photo.
- **Results in one fixed order,** processing order alone: the tool whose name starts with the text would sit among the rest; names first, then words, then darktable's, puts the likely one at the top.
- **Find Tool… as a utility with a section of its own:** a tool that edits nothing and shows only other tools, with a place in a tab, a header and a way to be hidden, none of which a search needs.
- **Find Tool… on Ctrl+F,** Search Photos… taking another key: Ctrl+F is the key everyone knows for finding, and searching photos is the more important search, so it keeps Ctrl+F.
- **A Tools panel scope for Ctrl+F,** finding a tool with the focus in the panel and photos elsewhere: one key meaning two things by where the focus is, against one scope for the view (37).

**Customise tabs:**

- **The dialog on fixed lists in Phase 2,** then reworked in Phase 3 (83): built twice, for what the file, ⋯ and the drag already do in Phase 2.

## Consequences

- **Decision 83** keeps its layout operations, display order only, its drag, its ⋯ and its floating panels; its storage, `hidden`, new-tool placement, invariants, Customise tabs and plan are this record's. **Decision 38**'s lists are the built-in workspaces' defaults and its "search may come later" is Find Tool…. **Decision 37**'s workspace is this one. **Decision 90**'s Export is never hidden. **Decision 91** gains the app's place `TabBar`. **Decision 92**'s `toolbar` list is kept in the workspace's file.
- `data/workspaces/`: the built-in workspaces' files, `default.toml` and `essentials.toml`, embedded by the app, each with its line in `data/README.md` and a test holding it to its lists, Default's decision 38's, and to the invariants.
- `crates/app`: `layout.rs` (the model, the file's reading and writing, resolving, the operations), the tabs drawn by id through one dispatcher, `panel-tab-1` to `-12`, `workspace-<id>` and the new actions with their messages in four languages, the Hidden tab, the dot, the search field and list, `Place::TabBar`.
- `apertrust-ui`: the search field in the tab bar's place, `eye-off` and its filled form, `square`, the dot on an icon button (as the Toolbar's).
- `data/settings.schema.toml`: `workspace`, `tools_panel.show_hidden`, `edit.tab` read only for its move; `state.toml`'s `[tab]`.
- Fluent: `tab-hidden`, `workspace-default`, `workspace-essentials`, Find Tool's and the layout items' words, each tool's `.search` with darktable's names and aliases.
- The Colour picker, hidden by default ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)), is a tool in no tab of the Default workspace: in the Hidden tab and found by search until its tab is decided.
- The agent kit's `ui-conventions` and `desktop-architecture` skills: a tool's ⋯ with Hide, the Hidden tab, the workspace's file.
