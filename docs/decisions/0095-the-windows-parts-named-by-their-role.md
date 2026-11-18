# 0095. The window's parts named by their role

## Context

The window's main parts were named after where they sit: the top panel, the side panel, the central panel and the grid panel, with the central bar and the grid bar inside them, and the Photos tab's first tool was Browse. The names reach people through the interface's words (screen readers' names, View › Panels, the menus, Settings' Side panel group), the settings keys already in people's `settings.toml`, the code and the docs.

Positional names have three problems:

- **They don't say what a part is for.** A photographer looks for the tools or the browser, not for a side panel.
- **They go stale when the layout moves.** The side panel sits at the right or the left, the grid panel at the bottom, the left or the right, and Touch mode's portrait layout shows the side panel as a bottom sheet ([0094](0094-one-interface-standard-and-touch-modes.md)).
- **"Side panel" undersells it.** Everything in it is a tool ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)).

What constrains the rename:

- One vocabulary in the interface, the docs and the code ([0036](0036-the-design-system.md)).
- `tools.*` already holds the tools' own settings, as `tools.exposure.lightness`.
- A settings key is permanent unless migrated: a key that moves raises the file's version and is moved on the TOML table before it is read ([0043](0043-settings-and-apertrusts-folders.md)).
- Nightly builds are in people's hands, so a renamed key must keep its value.

## Decision

Planned, in one rename pull request (below, The steps); until it lands, the window's parts keep their old names everywhere but this record.

### The names

| Part | Name | Was | What it is for |
| --- | --- | --- | --- |
| The bar along the top edge, the window's title bar | **Toolbar** | Top panel | The app's and tools' actions, as a person arranges them ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md), [0092](0092-the-toolbar.md)) |
| The panel of tabs and tools | **Tools**, the Tools panel | Side panel | Every processing tool and utility, in the workspace's tabs ([0038](0038-the-side-panels-tabs-by-editing-stage.md), [0083](0083-the-side-panels-layout-dragged-and-floating.md)) |
| The photo | **Viewer** | Central panel | The layout: the active photo, the selected photos side by side, later the map ([0037](0037-one-view-the-photos-tab.md)) |
| The thumbnails | **Browser** | Grid panel | The photos of the open folder, album or project |
| The Photos tab's first tool | **Collections** | Browse | Choosing what the Browser shows ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)) |

- **The bars follow their parts:** the viewer bar (was the central bar) and the browser bar (was the grid bar). A cell stays a cell, the Browser being a grid of them; the tab bar, sections, the caption and floating panels keep their names. "Panel" stays the kind: the panels are the window's main regions, bars the strips inside them.
- **Written as names,** capitalised as the tabs and tools are: the Toolbar, the Viewer, the Browser, Collections. In a list of the parts, View › Panels and the regions F6 visits and screen readers name, the panel of tools is **Tools**; in prose, and in a label where "Tools" alone would read as the tools themselves, it is **the Tools panel** ("Tools panel width"). The bars are written in lower case.
- **"Browser" alone is the part.** A web browser is written "web browser" wherever the two could be confused.
- **Collections** fits every mode: a folder's tree in folder mode, and Folders, Albums and Smart albums in a library or a project. darktable's module that chooses what its lighttable and filmstrip show is collections (`src/libs/collect.c`), and the docs already call any set of photos the tool chooses a collection. Folder, Album and Smart album stay the interface's words for what it holds; "collection" in lower case stays the code's and the docs' word for a set.
- **The start window's Browse…** is a verb, finding a library's or a project's file, and keeps its word.

### The interface's words

In British English, the source; American English, Ukrainian and Polish follow, Ukrainian and Polish as drafts awaiting native review as the others are ([0036](0036-the-design-system.md)). A Fluent id follows its words, as the table shows; every other id keeps its name, `grid-` among them where it names the cells, their rows, columns, sort and size (`grid-cell-*`, `grid-rows`, `grid-sort`, `grid-thumbnail-size`).

| Id now | New id | Words now | New words |
| --- | --- | --- | --- |
| `top-panel` | `toolbar` | Top panel | Toolbar |
| `top-panel-menu`, `top-panel-more` | `toolbar-menu`, `toolbar-more` | Menu, More actions | unchanged |
| `panel-tabs` | `tools-panel-tabs` | Side panel tabs | Tools panel tabs |
| `panel-width` | `tools-panel-width` | Side panel width | Tools panel width |
| `menu-panel-tab` | `menu-tools-panel-tab` | Side Panel Tab | Tools Panel Tab |
| `action-panel-tab-1` to `-7` | unchanged | Side-panel tab 1 | Tools panel tab 1 |
| `settings-group-side-panel` | `settings-group-tools-panel` | Side panel | Tools panel |
| `central-bar` | `viewer-bar` | Central bar | Viewer bar |
| `grid-panel` | `browser` | Grid panel | Browser |
| `grid-panel-height`, `grid-panel-width` | `browser-height`, `browser-width` | Grid panel height, width | Browser height, width |
| `grid-bar` | `browser-bar` | Grid bar | Browser bar |
| `grid-bar-options` | `browser-bar-options` | Grid panel options | Browser options |
| `grid-position`, `menu-grid-position` | `browser-position`, `menu-browser-position` | Position, Grid Panel Position | Position, Browser Position |
| `action-grid-position-*` | `action-browser-position-*` | unchanged | unchanged |
| `section-browse` | `section-collections` | Browse | Collections |
| `action-show-browse` | `action-show-collections` | Browse (View › Browse, L) | Collections (View › Collections, L) |
| every `browse-*` message | `collections-*` | unchanged | unchanged |
| `folder-empty-hint` | unchanged | Choose a subfolder in Browse. | Choose a subfolder in Collections. |
| `setting-side-panel-*`, `setting-central-panel-*`, `setting-grid-panel-*` | `setting-tools-panel-*`, `setting-viewer-*`, `setting-browser-*` | unchanged | unchanged |
| `start-browse` | unchanged | Browse… | unchanged |

### What files outside the build hold

**Settings keys.** The Tools panel's keys take `tools_panel.*`, since `tools.*` is the tools' own; the Viewer's and the Browser's take their names alone.

| Key now | New key |
| --- | --- |
| `side_panel.tab_names` | `tools_panel.tab_names` |
| `side_panel.width` | `tools_panel.width` |
| `central_panel.margin` | `viewer.margin` |
| `central_panel.proof_margin` | `viewer.proof_margin` |
| `central_panel.background` | `viewer.background` |
| `grid_panel.sort` | `browser.sort` |
| `grid_panel.descending` | `browser.descending` |
| `grid_panel.thumbnails` | `browser.thumbnails` |
| `grid_panel.labels` | `browser.labels` |
| `grid_panel.square_thumbnails` | `browser.square_thumbnails` |
| `grid_panel.position` | `browser.position` |
| `grid_panel.rows` | `browser.rows` |
| `grid_panel.columns` | `browser.columns` |

- **The Settings dialog's group** on Editing, `group = "side_panel"`, becomes `tools_panel`.
- **Planned keys take the new names from the start:** `toolbar.labels` ([0092](0092-the-toolbar.md)); `tools_panel.sections`, `tools_panel.groups` and `tools_panel.floating` ([0083](0083-the-side-panels-layout-dragged-and-floating.md)).
- **Each generated static and computed default follows its key:** `SIDE_PANEL_WIDTH` becomes `TOOLS_PANEL_WIDTH`, `computed::side_panel_width` `tools_panel_width`, `central_panel_background` `viewer_background`.

**Action ids,** which `[keys]` holds by id: `show-browse` becomes `show-collections`, and `grid-position-left`, `-right` and `-bottom` become `browser-position-left`, `-right` and `-bottom`. `panel-tab-1` to `-7` keep their ids, "panel" being the kind.

**Browse's id,** `browse`, becomes `collections`: the tool's and its section's id, which `edit.sections` and `edit.groups` hold by id (`edit.groups.browse.folders` becomes `edit.groups.collections.folders`).

**The themes' two sizes** named after parts, `top_bar` and `side_panel`, become `toolbar` and `tools_panel` in the built-in theme files, and a theme file that still says the old name loads, the old name read as the new one.

### The migration

`settings.toml` becomes version 2, the store's first `Migration`, from version 1, run as every migration is: on the file's table before it is read, and before each write merges into it ([0043](0043-settings-and-apertrusts-folders.md)).

- **It moves** each old key to its new key, each old action id's entry in `[keys]` to its new id, and the `browse` entries of `edit.sections` and `edit.groups` to `collections`.
- **Where both are present,** the new key wins and the old is dropped, with a line under `-d settings` naming it. Only a hand edit puts a new key into a version-1 file, since a version-2 build never writes version 1.
- **Nothing is lost:** a file is read migrated at every start, and written as version 2, holding the new keys alone, at the next write, which keeps every value.
- **An older build** reads a version-2 file for what it knows and never overwrites it ([0043](0043-settings-and-apertrusts-folders.md)), so it shows these settings at their defaults. No release has been made, only nightly builds, so the cost is a nightly build older than the rename.
- **`--conf` takes the new keys only.** An old one stops the start, naming it, as any key no setting has does.
- **The test:** a version-1 file holding every renamed key, action id and `browse` entry reads with every value kept and writes as version 2 with the new keys alone; one holding an old key and its new one keeps the new value. The round trip of every key, byte for byte, is of a version-2 file.

### The code

Every identifier that names a part's old name follows its new name; "panel" alone, the kind, stays (`PanelTab`, `PanelEdge`, `PanelEdgeAction`, `PanelBody`, `TabBar`), and so does "grid" where it names the cells (`GridCell`, `GridCellOutput`, `grid_cell`).

| Crate | Now | New |
| --- | --- | --- |
| `apertrust-ui` | `components::top_panel`, `TopPanel`, `TopPanelAction` | `components::toolbar`, `Toolbar`, `ToolbarAction` |
| `apertrust-ui` | `components::side_panel` | `components::tools_panel` |
| `apertrust-ui` | `components::central_bar`, `CentralBar`, `CentralBarAction` | `components::viewer_bar`, `ViewerBar`, `ViewerBarAction` |
| `apertrust-ui` | `components::grid_panel`, `GridPanel`, `GridPanelAction`, `GridPanelOutput`, `GridBarAction`, `GridPosition` | `components::browser`, `Browser`, `BrowserAction`, `BrowserOutput`, `BrowserBarAction`, `BrowserPosition` |
| `apertrust-ui` | `grid_panel_height`, `grid_panel_width`, `grid_panel_thickness`, `grid_panel_lines` | `browser_height`, `browser_width`, `browser_thickness`, `browser_lines` |
| `apertrust-ui` | `components::grid_edge`, `GridEdge`, `GridEdgeAction` | `components::browser_edge`, `BrowserEdge`, `BrowserEdgeAction` |
| `apertrust-ui` | `Sizes::top_bar`, `Sizes::side_panel`; the gallery's `show_side_panel` | `Sizes::toolbar`, `Sizes::tools_panel`; `show_tools_panel` |
| `apertrust-tools` | `Widget::Recents`, `Widget::Folders`; the utility `browse` | `Widget::CollectionsTopLine`, `Widget::CollectionsGroups`, as Export's widgets are prefixed; `collections` |
| `apertrust` (app) | `FromTopPanel`, `TOP_PANEL`, `fn top_panel` | `FromToolbar`, `TOOLBAR`, `fn toolbar` |
| `apertrust` | `fn side_panel`, `fn central_panel`, `fn grid_panel`, `fn grid_edge` | `fn tools_panel`, `fn viewer`, `fn browser`, `fn browser_edge` |
| `apertrust` | `settings::CentralPanel` | `settings::Viewer` |
| `apertrust` | `library::Browse`, `fn toggle_browse`, `fn browse_open` | `library::Collections`, `fn toggle_collections`, `fn collections_open` |
| `apertrust` | `actions::SHOW_BROWSE`, `actions::GRID_POSITION` | `actions::SHOW_COLLECTIONS`, `actions::BROWSER_POSITION` |
| planned | `Place::TopPanel` ([0091](0091-context-menus-places-and-tools-items-as-actions.md), [0092](0092-the-toolbar.md)) | `Place::Toolbar` |

- **Tests and snapshots** that name a part follow it: `the_grid_panel_draws_only_what_is_visible` becomes `the_browser_draws_only_what_is_visible`, the snapshot `grid-panel-left` `browser-left`.
- **Left as they are,** naming something else: `apertrust-library`'s `browser_order`, the order a file browser lists names in; `storage::Folders` and `folders::Folders`, folders on disk; `recents::Recents`, the recents' list.

### The steps

1. **This decision,** with decision 37's paragraph naming the parts, the index, the roadmap and AGENTS.md, all marked planned; until step 2 lands, the code, the interface's words and every other doc, the other decision records included, keep the old names, which the tables above map.
2. **One rename pull request,** planned, so no nightly build sits between renamed keys and their migration, in commits by layer, each building and passing its tests:
   1. the code's identifiers;
   2. the words and their Fluent ids in en-GB, en-US, uk and pl;
   3. the settings keys, the action ids and Browse's id, with the migration to version 2 and its test;
   4. the themes' two sizes renamed in the built-in theme files, the theme loader still reading the old names (no migration: a theme file is read, never written);
   5. the docs: the decision records that name the parts (below), architecture.md, gui.md, library.md, settings.md, crates.md, the tool docs and the rest, about 500 mentions, their anchors with them.

   It changes no behaviour and no look beyond the words: the snapshots change only where a part's name is drawn.

Then the UI wireframe's boards, and the agent kit's `ui-conventions` and `desktop-architecture` skills, a pull request to the kit.

## Options considered

**For the names:**

- **The positional names kept:** they say nothing of a part's role and go stale as the layout moves.
- **Filmstrip for the Browser,** darktable's word: a filmstrip is one row, where the Browser is a grid at the bottom or a column at a side.
- **Grid for the Browser:** it names the shape, which the Viewer's Photo layout shares with several photos selected.
- **Library or Photos for the Browser:** Library is the Phase 5 mode and Photos the tab ([0037](0037-one-view-the-photos-tab.md)).
- **Photo or Canvas for the Viewer:** Photo is the layout's name, and a canvas is painted on.
- **Inspector or Panel for Tools:** neither says what it holds, and "panel" is the kind.
- **Browse kept beside the Browser:** two names a letter apart, a verb among the tools' nouns.
- **Folders, Sources or Places for Collections:** Folders fits folder mode alone, an album is neither a source nor a place ([0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md)).

**For the keys and the migration:**

- **`tools.*` for the Tools panel's keys:** it holds the tools' own settings, and a part's key would sit among the tools' ids.
- **`viewer_panel.*` and `browser_panel.*`:** "panel" repeated where the name alone is clear; the Tools panel needs it, the others do not.
- **The keys kept as they are:** the old names would stay in every `settings.toml` and in `--conf`, one vocabulary broken where people read it.
- **An alias for each key, no new version:** the schema naming each setting's old key, the old read where the new is missing and left for older builds. Older builds would keep working, but each would write a key the other no longer reads, and [0043](0043-settings-and-apertrusts-folders.md)'s rule raises the version for a key that moves.
- **The old key winning where both are present:** a stale value over one typed by hand for the new name.
- **`--conf` taking the old keys too:** a second name for every key, for scripts that no release has given.
- **The themes' sizes kept under their old names:** one vocabulary broken in the theme files, where reading an old name costs one line.
- **The rename and the migration in two pull requests:** the nightly builds between them would lose these settings.

**For the records:**

- **Each retitled record's file renamed:** every link to it across the docs would break, where [0066](0066-one-record-for-each-decision.md) keeps numbers and links; the files keep their names, as `0075`'s already differs from its title.

## Consequences

Each of these is planned, with the rename pull request:

- **[0037](0037-one-view-the-photos-tab.md)** names the parts by their role, and its words add Collections and the parts' names, "collection" staying the code's and the docs' word for a set.
- **[0073](0073-the-photos-tabs-tools-browse-first-filters-in-folder-mode.md):** Collections is the Photos tab's first tool, its id `collections`, View › Collections (`show-collections`, L); Browse is among the options it rejects.
- **The records that name the parts say the new names once the rename lands,** in its docs commit: 4, 38, 39, 43, 46, 57, 69, 77, 78, 79, 81, 83, 85, 86, 88, 89, 90, 91, 92, 93 and 94, their code names this decision's. The titles of 38, 39, 73, 83 and 92 change; their files keep their names, so every link stays.
- **The settings store** gains its first migration, `settings.toml` version 2 ([0043](0043-settings-and-apertrusts-folders.md)), and settings.md's test of a version-1 file becomes one of version 2.
- **gui.md's "The names of the parts"** lists the new names, the code's types and messages with them, and drops its note that `top_bar` keeps its name.
- **The docs' sweep** is the rename pull request's: until it lands, gui.md, library.md, settings.md, crates.md, the tool docs and the port specs say the old names.
