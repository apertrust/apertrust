# 0093. One tool model: processing tools and utility tools

## Context

Everything in the Tools panel is a tool ([0004](0004-tools-for-people-operations-for-algorithms.md)), and the tools that edit the photo and those that edit nothing look mostly the same: a header with ↺, presets and ⋯, a page of rows, items for context menus and the toolbar ([0091](0091-context-menus-places-and-tools-items-as-actions.md), [0092](0092-the-toolbar.md)). A tool that edits the photo is a declaration in `apertrust-tools`, a `ProcessingDefinition` with a page of `Row`s, and an implementation of the trait `ProcessingTool` ([0028](0028-tools-bound-to-operations.md)), which `crates/app/src/tools.rs` draws with no code of its own. A utility coded apart, each app module (`history.rs`, `scopes.rs`, `library.rs` for Collections, `export.rs`) building its own `Section` header and page, lets the headers drift apart, gives a utility's ↺, presets and ⋯ whatever behaviour its module makes, and leaves Export's rows, which are settings, in code no other tool can share. darktable has the same split, and joins its two kinds by composition: its processing modules (`dt_iop_module_so_t`, `src/iop/iop_api.h`) and its utility modules (`dt_lib_module_t`, `src/libs/lib_api.h`) both begin with one embedded struct, `dt_action_t`, which names them for shortcuts and actions, and each kind has an API of its own. The question is what the two kinds are called, and how one model in the code covers both.

## Decision

**One tool model: a tool is the parent, a processing tool and a utility tool its two kinds, sharing one declaration and one header.** Built, in six steps (below); a utility's presets are planned with one presets API every tool shares, as darktable's are (`src/common/presets.c`, `src/gui/presets.c`), a decision of its own.

### The terms

| Term | Is | darktable's |
| --- | --- | --- |
| **Tool** | The parent: everything a person works with in the Tools panel, and the Import sheet's Import tool, with one header, one page and one ⋯ | module |
| **Processing tool** | A tool that edits the photo through operations: Exposure, AgX, Colour calibration… | processing module (iop) |
| **Utility tool**, "utility" for short | A tool that edits nothing of its own, with no controls and no operation: Collections, Filters, Keywords, Metadata, Styles, Export, Scopes, History, Snapshots, the Colour picker, and the Import sheet's Import tool ([0086](0086-import-as-a-full-window-sheet.md)) | utility module (lib) |

- **What a utility holds is what it is** ([0004](0004-tools-for-people-operations-for-algorithms.md)): settings, view state or the photo's metadata, never controls in the edit. A utility may send the session's commands, as History's `Reset`, `GoTo` and `Compress` do ([0078](0078-undo-redo-and-the-edits-history.md)), but binds no operation.
- **"Section" no longer names the concept.** It stays only for the Tools panel's block, a header with its body: `apertrust-ui`'s `Section` component and the layout's section ids ([0083](0083-the-side-panels-layout-dragged-and-floating.md)). The menus' `Section` constants ([0091](0091-context-menus-places-and-tools-items-as-actions.md)) name a menu's section and keep their name.
- **No word a person sees changes.** The interface already says tool, and a utility's tooltip already says "Edits nothing of its own".

### The code: composition, as darktable's

| darktable | Apertrust |
| --- | --- |
| `dt_action_t`, first in both | `ToolDefinition`, the parent, embedded in both |
| `dt_iop_module_so_t` with `iop_api.h` | `ProcessingDefinition` with the trait `ProcessingTool` |
| `dt_lib_module_t` with `lib_api.h` | `UtilityDefinition` with the trait `UtilityTool` |

```rust
// apertrust-tools
/// What every tool declares: the parent.
pub struct ToolDefinition {
    pub id: &'static str,
    pub tabs: &'static [Tab],             // the page: tabs, groups, rows
    pub before: &'static [Row],
    pub after: &'static [Row],
    pub reset: Reset,                     // the header's ↺
    pub presets: bool,                    // the header's presets button
    pub open_at_first: bool,              // its section open until the person chooses
    pub menus: &'static [MenuItem],       // context menus and ⋯ (decision 91)
    pub toolbar: &'static [ToolbarItem],  // decision 92
    pub cursors: &'static [ToolCursor],   // a tool's picker cursors (decision 104)
}
// ToolDefinition::processing(id, tabs): a processing tool's defaults, closed at first among them

/// A processing tool's declaration.
pub struct ProcessingDefinition {
    pub tool: ToolDefinition,
    // version, layers, auto, switched, controls, preset_rows, views, pickers, facts…
}

/// A utility tool's declaration.
pub struct UtilityDefinition {
    pub tool: ToolDefinition,
    pub measures: Option<WorksOn>,
}

/// A processing tool's contract with the session: iop_api.h's non-GUI half.
pub trait ProcessingTool: Send + Sync {
    fn definition(&self) -> &'static ProcessingDefinition;
    // defaults, owns, bind, available, options, range, curve, pick, action…, as today
}

// the app
/// A utility tool's behaviour: lib_api.h's GUI half.
pub trait UtilityTool {
    fn definition(&self) -> &'static UtilityDefinition;
    fn body(&mut self, widget: Widget, ui: &mut egui::Ui, cx: &mut Context) -> Vec<Action>;  // one of its custom rows
    fn reset(&mut self, cx: &mut Context) {}
    // later: overlay and pointer, for a utility that draws over the photo
}
```

- **The parent is data both kinds embed,** as darktable's `dt_action_t` is; behaviour is an API for each kind. The fields every tool shares move into `ToolDefinition`; what editing needs stays in `ProcessingDefinition`; a utility adds only `measures`.
- **`reset`** says what the header's ↺ does: a processing tool's own reset (`ResetTool`); a registry action, as History's `reset`, which Edit › Reset runs too; the utility's own `UtilityTool::reset`, as Export's Default settings; or none, as Scopes and Collections.
- **`open_at_first`** says whether its section is open before the person opens or closes it: every processing tool is closed at first, as darktable's processing modules start collapsed; of the utilities, Collections, the way to the photos, Scopes and History are open, and Export closed, Export… and Ctrl+E opening it. A state the person chose, kept app-wide by the tool's id in `settings.toml`'s `[edit.sections]`, always wins over it. Every header reads it through the shell, so no module of the app holds a default of its own.
- **`presets`** is the header's presets button. A processing tool's preset rows ([0031](0031-views-and-curves-of-the-same-stored-values.md)) are `ProcessingDefinition::preset_rows`, so the two never share a name.
- **A tool's place** ([0086](0086-import-as-a-full-window-sheet.md), planned), the Tools panel or Import, is a field of `ToolDefinition` when it is built, since every tool has one.
- **`ProcessingTool` stays in `apertrust-tools`,** run by the engine's session, with no egui ([0028](0028-tools-bound-to-operations.md)). **`UtilityTool` lives in the app,** since a utility's behaviour is its widgets: `apertrust-tools` keeps no egui, and the engine never sees a utility.
- **One `Context` for every utility:** the mirror, the settings, the words and a way to send commands, the frame's command list. History's steps, Collections' folder, Scopes' measurements and Export's jobs are in the mirror, and Export's choices in the settings, so one context covers them all. `body` is required; `reset` and every later method are provided as no-ops.
- **`UTILITIES` lists every utility's `UtilityDefinition`,** Collections among them, as `TOOLS` lists the processing tools.

### One header, one page drawer, one ⋯

- **The app draws every tool's header, page, ⋯ menu, open state and layout from `&ToolDefinition`,** so ↺, presets and ⋯ behave alike in every tool. This departs from darktable, which builds the header twice (`dt_iop_gui_set_expander`, `dt_lib_gui_get_expander`).
- **`apertrust-ui` keeps no tool types** ([0036](0036-the-design-system.md)): the app turns a `ToolDefinition` into the design system's `Section` and rows.
- **The header's buttons follow the declaration:** ↺ where `reset` is not none, presets where `presets` is true, ⋯ always, holding the tool's declared `MenuItem`s at `Place::Options` before the layout's items (decision 91). History's Compress History, Truncate History, Create a Style… and Discard History… are such declared items, four, ⋯'s most, Truncate History an item of its own, so a declared item stays one action and `MenuEntry` needs no alternate.

### A utility's page

- **A utility uses the same `Row`s** as a processing tool, on one untitled tab, since its header draws no tab strip. `UtilityDefinition::check` refuses every row that edits: a row naming a control (a slider, a choice, a toggle, `Row::When`, a control's row of buttons, `Row::Buttons`, a custom row naming controls), a preset row, a picker's rows (`Row::Pickers`, `Row::Patches`, `Row::Reference`, `Row::LastPick`), a computed value and the guides, a processing tool's, and a custom row of a processing tool's widget, as `ProcessingDefinition::check` refuses a utility's. A utility's rows are settings, labels, groups, `Row::WhenSetting`, actions, each running a registry action, and custom rows naming no control. Its ↺ is never `Reset::Tool`. A utility's action rows name registry actions, each with its own availability, never a generated `tool-<tool>-<action>`, whose rule, a photo that takes edits, is a processing tool's ([0091](0091-context-menus-places-and-tools-items-as-actions.md)): Import's Use $(MONTH) is available while the sheet shows the problem it fixes, with or without a photo in the window. Until a utility's action row is bound to the registry, the check refuses it. A utility may declare `presets`: its header's presets button, named `<id>-presets`, is drawn disabled, its tooltip `<id>-presets-later`, until its presets come with the presets API every tool shares (planned); its own ↺ is named `<id>-reset`.
- **All variation goes into custom widgets.** `Row::Custom { widget }`, as the tone equalizer's graph is today, also covers History's steps, Collections' recents line and its Folders, the scope graph with its buttons and Export's size line and path line, each a `Widget` of the closed registry. The page drawer draws every other row itself and, reaching a utility's custom row, calls that utility's `UtilityTool::body` with the row's `Widget`, once per custom row: Collections' `body` is called for its recents line and again for its Folders.
- **A utility's widget keeps what it draws today.** Scopes' graph draws its buttons over itself, shown while the pointer is over it: which graph, the scale, the channels and the clipping toggles are the graph's own state, kept in the settings' `scopes.*`, never setting rows of the page. Collections' Folders is its own group, holding the folder tree in a list of fixed height that scrolls under the recents line, so Collections keeps its height whatever the tree holds, its title `collections-folders` and its open state as the settings keep its groups. Export's custom rows are Target storage, which no setting holds yet, the path line, the size line, the groups still to come and the Summary with the tool's end, each drawing what `export.rs` drew; its settings are setting rows. Each utility matches its own widgets in `body`, whose egui code stays in its app module, takes that utility's state and returns its own actions. A processing tool's custom rows, as the tone equalizer's graph, stay drawn by the app's own `match` on `Widget`, as today.
- **A utility with presets has presets of its stored settings,** `presets` true in its `ToolDefinition`: Export for now, History, Scopes and Collections having none. They are presets of the settings its page stores, those its setting rows hold and those its custom rows hold alike (Export's size line sets the width and height For sharing gives), as darktable's `get_params` and `set_params` are presets of a utility module's state, with no binary blob. Export's are the first ([0090](0090-export-as-a-photos-tab-tool.md)). They are planned, with one presets API every tool shares, processing and utility alike, as darktable's are (`presets.c`, `gui/presets.c`), a decision of its own; until then the presets button is drawn disabled.
- **A setting row may name its key in full** (`SettingRow::key`) and declare its words (`SettingRow::words`: its label, full name, promise and options, an option a message or a name written alike in every language), as Export's rows name `export.*` settings by their keys as strings with their `export-*` words; a boolean whose words name two options is darktable's choice of No and Yes. The app binds them to `apertrust-export`'s settings; a test fails on a key that is not in the settings schema. A processing tool's settings name neither, `ProcessingDefinition::check` refusing them. `apertrust-tools` does not depend on `apertrust-export`, as it depends on no operation crate, and `apertrust-export` stays its own crate ([0056](0056-export-as-an-engine-job.md)).
- **What a utility gives its page drawer,** through `UtilityTool`, beside its custom rows: its keyed settings' values (`setting`), a change of one routed to it by the app (`set`), never the tools' settings or `PickerCommand::Settings`; options a setting row draws disabled now, with why (`disabled`); a tooltip in place of a row's promise, its description too (`tip`); a note under a row, a problem in `error` with its fix (`note`, `fix`); how many settings its custom rows hold off their defaults, which a collapsible group counts with its setting rows (`set_in`); and its rows' spacing (`spacing`). Its declaration titles its labels and groups where they are not the ones their ids imply, and says which groups open at first (`UtilityDefinition::titles`, `open`).

| Tool | Its page |
| --- | --- |
| Exposure, AgX and the other processing tools | As now |
| Tone equalizer | As now, its graph a custom row |
| Export | Groups of setting rows (Storage options, Format options and Global options of [0090](0090-export-as-a-photos-tab-tool.md)), format-specific rows shown only for their formats (`Row::WhenSetting`); Target storage, the path line, the size line, the groups to come and the Summary with the tool's end custom rows; ↺ its own; its presets button drawn disabled until the shared presets API |
| Scopes | The graph with its buttons a custom row, which graph the graph's own state |
| History | The steps a custom row; ↺ the `reset` action; ⋯ holding Compress History, Truncate History, Create a Style… and Discard History… as declared items |
| Collections | Two custom rows: the recents line, and Folders, its group holding the tree in its scrolling list |

### darktable's `lib_api.h`, and where each part goes

| darktable | What it does | Apertrust |
| --- | --- | --- |
| `name`, `description`, `expandable`, `has_preset_label` | Identity and header | `ToolDefinition` |
| `views`, `container`, `position` | Where it sits | The workspace's layout ([0083](0083-the-side-panels-layout-dragged-and-floating.md)) |
| `gui_init`, `gui_cleanup`, `gui_update` | Builds its widgets | `UtilityTool::body`, each custom row drawn each frame |
| `gui_reset` | The header's ↺ | `ToolDefinition::reset`, `UtilityTool::reset` |
| `get_params`, `set_params`, `legacy_params`, `init_presets`, `manage_presets`, `preset_autoapply` | Presets of its state | Presets of every setting its page stores, its setting rows' and its custom rows' |
| `set_preferences` | Items in its presets menu | Its ⋯ items ([0091](0091-context-menus-places-and-tools-items-as-actions.md)) |
| `gui_tool_box` | A widget in the top bar | Its toolbar items ([0092](0092-the-toolbar.md)) |
| `gui_post_expose`, `mouse_moved`, `button_pressed`, `scrolled` | Draws over the photo, takes the pointer | Later: `UtilityTool::overlay` and `pointer` |
| `view_enter`, `view_leave` | View changes | None: there is one view ([0037](0037-one-view-the-photos-tab.md)) |

### The steps

Each step is reviewed before the next.

1. **93.1, the renames and the parent,** built: mechanical, with no behaviour change: `Definition` to `ProcessingDefinition`, `Utility` to `UtilityDefinition`, the trait `Tool` to `ProcessingTool` (about 250 uses), `Definition::presets` to `preset_rows`, and the shared fields into `ToolDefinition`, `Utility::menus` among them. The docs' names follow in the same pull request.
2. **93.2, `UtilityTool` in the app,** built, with History, Scopes and Collections implementing it, Collections joining `UTILITIES`, `main.rs` calling them through the trait.
3. **93.3, one header and ⋯** for every tool in `TOOLS` and `UTILITIES`, built with 93.2: `ToolDefinition::reset` and `presets`, the header drawn from `&ToolDefinition`, History's ⋯ items declared with it, Truncate History an item of its own, so its ⋯ never goes empty; Export's with step 93.5.
4. **93.4, the utilities on the page model,** built: one page drawer for every tool's page and `UtilityDefinition::check`, with Scopes, the smallest, first, then History, then Collections.
5. **93.5, Export on the tool model,** built: Export implements `UtilityTool` and joins `UTILITIES`, drawn with the shared header, its ↺ its own and its presets button drawn disabled, its page custom rows drawing what `export.rs` drew, so nothing a person sees changes.
6. **93.6, Export's page as setting rows,** built, before Import's sheet shares those rows: groups of setting rows naming the `export.*` settings by their full keys, a test failing on a key not in the settings schema, the size line and the path line custom rows; its presets come with the presets API every tool shares, planned. The page model grows for it, each part keeping what Export shows, its snapshots byte for byte:
   - setting rows naming a key in full, with their words declared, Export's `export-*`;
   - a utility's settings kept in the settings store, never sent through `PickerCommand::Settings` as a processing tool's are; an export carries them to the session (decision 90);
   - options a setting row draws disabled at run time, as On conflict's Overwrite where the platform cannot ask first, and the unbuilt storages and formats;
   - a No and Yes style for a boolean setting, darktable's;
   - tooltips and notes under a row that the utility supplies, as the size's and the path's from the session's preview;
   - settings counted in a collapsible group's "N set", as controls are;
   - Export's header drawn through the shell's head as the other utilities' are, with its `photo` set, so that a choice in its ⋯ made for another photo is dropped once its ⋯ has items.

## Options considered

- **Two models, as today:** each utility draws its own header and page, so ↺, presets and ⋯ drift apart, Collections is in no list, and Export's setting rows, presets and the Import sheet's Import tool, all settings, would each be drawn by code of their own.
- **Each app module's own default for its section's open state,** a constant beside each utility and one for every processing tool: the defaults drift apart from the declaration that names the tool, and a client other than the app could not read them.
- **One flat definition for both,** a utility leaving the editing fields empty: a utility would carry a version, layers, Auto, switched, controls and pickers that mean nothing for it, each needing a check that it is empty; darktable keeps the two apart.
- **One trait for both kinds,** in `apertrust-tools`: a utility's behaviour is its widgets, which needs egui there, against [0028](0028-tools-bound-to-operations.md); the processing half is what the engine runs, which a utility never is.
- **A shared header only,** each utility's page still written by hand: the header would agree, but setting rows, their presets and a check on what a utility may show would not be shared, and Export's and Import's rows would stay two codes.
- **The header built per kind, as darktable's two expanders:** two codes to keep alike for one block that looks the same.
- **`apertrust-tools` depending on `apertrust-export`,** so Export's rows name typed settings: it couples the tool layer to one utility's crate and its writers; keys as strings with a test against the schema keep the boundary and fail as early.
- **Truncate History kept as Compress History's alternate,** shown while Option is held: a declared item would need to name a second action, a field of `MenuEntry` for one item, and the alternate is hidden from anyone who doesn't know to hold Option.
- **Which scope as a setting row of Scopes' page:** a setting row is a row of its own, so the buttons over the graph would move off it, a change of look, unless a custom row drew the settings it names, a second path for a setting's words and key; and the scale, the channels and the clipping toggles, settings of the same kind, would stay the graph's.
- **Collections' Folders as a group the page drawer draws,** the tree its custom row: the drawer's groups take `tool-<tool>-collapsible-<group>` for their title, where Folders' is `collections-folders`, and no row holds rows in a list that scrolls under the rows before it; both would need a new kind of row or words, for one group.
- **Export's page as setting rows in the step that puts it on the tool model:** the page model draws none of what its rows need (keys in full with Export's words, settings kept in the store, options disabled at run time, No and Yes, tooltips and notes from the session, settings counted in a group), so its look would change; step 93.6 grows the model for it, before Import's sheet shares the rows.
- **Keeping "section" for utilities,** or "utility" as a level beside tools: two names for what the Tools panel shows, moves and floats alike, rejected already in [0004](0004-tools-for-people-operations-for-algorithms.md).

## Consequences

- **[0004](0004-tools-for-people-operations-for-algorithms.md)** names the parent and its two kinds; **[0028](0028-tools-bound-to-operations.md)** describes `ProcessingDefinition` and `ProcessingTool` over the shared parent; **[0086](0086-import-as-a-full-window-sheet.md)** puts a tool's place in `ToolDefinition`; **[0090](0090-export-as-a-photos-tab-tool.md)** declares Export's page as a utility's setting rows and custom rows; **[0091](0091-context-menus-places-and-tools-items-as-actions.md)** and **[0092](0092-the-toolbar.md)** declare every tool's items in `ToolDefinition::menus` and `ToolDefinition::toolbar`.
- `apertrust-tools` holds `ToolDefinition`, `ProcessingDefinition`, `UtilityDefinition`, `ProcessingTool`, `UTILITIES` with every utility and `UtilityDefinition::check`; `Widget` gains the utilities' widgets. The app holds `UtilityTool`, its `Context`, the one header and page drawer, and each utility's widgets in its module. No crate is added, and no dependency changes.
- Nothing a person sees changes but two things, and no setting, no Fluent id, no session command and nothing in the sidecar: each utility keeps its look on the page model, its custom widgets drawing what its module draws today. The first is History's ⋯, where Truncate History is an item of its own from step 93.3, with the words it had; the second, every processing tool closed at first (`open_at_first`, below). A change of a utility's look is a design question of its own. The engine and its session are untouched but for the trait's name.
- `open_at_first` is the one default of a section's open state: a person who has never opened or closed a processing tool sees each closed, and any state kept in `[edit.sections]` stays as it was. Tests hold each tool's declared default, every processing tool closed and the four utilities as listed, and a kept state winning over it both ways.
- Every step is built: the new names, step 93.1, `UtilityTool` with the one header and ⋯, steps 93.2 and 93.3, the utilities on the page model, step 93.4, Export on the tool model, step 93.5, and Export's page as setting rows, step 93.6. A utility's presets are planned with the presets API every tool shares. [gui.md](../gui.md#one-tool-model) describes the model, [the roadmap](../roadmap.md) holds the steps.
- The agent kit's `tool-bindings`, `ui-conventions` and `desktop-architecture` skills are to state the terms and the model.
