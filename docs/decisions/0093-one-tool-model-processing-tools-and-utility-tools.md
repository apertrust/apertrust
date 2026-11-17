# 0093. One tool model: processing tools and utility tools

## Context

Everything in the side panel is a tool ([0004](0004-tools-for-people-operations-for-algorithms.md)), and the tools that edit the photo and those that edit nothing look mostly the same: a header with ↺, presets and ⋯, a page of rows, items for context menus and the toolbar ([0091](0091-context-menus-places-and-tools-items-as-actions.md), [0092](0092-the-toolbar.md)). They are coded in two ways:

- **A tool that edits the photo** is a declaration in `apertrust-tools`, a `ProcessingDefinition` with a page of `Row`s, and an implementation of the trait `ProcessingTool` ([0028](0028-tools-bound-to-operations.md)); `crates/app/src/tools.rs` draws every one of them with no code of its own.
- **A utility** is drawn wholly by its own app module (`history.rs`, `scopes.rs`, `library.rs` for Browse, `export.rs`), each building its own `Section` header, and `main.rs` calls each one by hand. `utility::UTILITIES` lists only Scopes and History, as `UtilityDefinition { tool, measures }`; Browse is in no list.

So the headers drift apart, a utility's ↺, presets and ⋯ behave as its module makes them, and Export's rows, which are settings, are drawn by code no other tool can share. darktable has the same split, and joins its two kinds by composition: its processing modules (`dt_iop_module_so_t`, `src/iop/iop_api.h`) and its utility modules (`dt_lib_module_t`, `src/libs/lib_api.h`) both begin with one embedded struct, `dt_action_t`, which names them for shortcuts and actions, and each kind has an API of its own. The question is what the two kinds are called, and how one model in the code covers both.

## Decision

**One tool model: a tool is the parent, a processing tool and a utility tool its two kinds, sharing one declaration and one header.** Planned, in five steps (below), the first three built.

### The terms

| Term | Is | darktable's |
| --- | --- | --- |
| **Tool** | The parent: everything a person works with in the side panel, with one header, one page and one ⋯ | module |
| **Processing tool** | A tool that edits the photo through operations: Exposure, AgX, Colour calibration… | processing module (iop) |
| **Utility tool**, "utility" for short | A tool that edits nothing of its own, with no controls and no operation: Browse, Filters, Keywords, Metadata, Styles, Export, Scopes, History, Snapshots, the Colour picker, and the Import sheet's import tools ([0086](0086-import-as-a-full-window-sheet.md)) | utility module (lib) |

- **What a utility holds is what it is** ([0004](0004-tools-for-people-operations-for-algorithms.md)): settings, view state or the photo's metadata, never controls in the edit. A utility may send the session's commands, as History's `Reset`, `GoTo` and `Compress` do ([0078](0078-undo-redo-and-the-edits-history.md)), but binds no operation.
- **"Section" no longer names the concept.** It stays only for the side panel's block, a header with its body: `apertrust-ui`'s `Section` component and the layout's section ids ([0083](0083-the-side-panels-layout-dragged-and-floating.md)). The menus' `Section` constants ([0091](0091-context-menus-places-and-tools-items-as-actions.md)) name a menu's section and keep their name.
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
    pub menus: &'static [MenuItem],       // context menus and ⋯ (decision 91)
    pub toolbar: &'static [ToolbarItem],  // decision 92
}

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
- **`reset`** says what the header's ↺ does: a processing tool's own reset (`ResetTool`); a registry action, as History's `reset`, which Edit › Reset runs too; the utility's own `UtilityTool::reset`, as Export's Default settings; or none, as Scopes and Browse.
- **`presets`** is the header's presets button. A processing tool's preset rows ([0031](0031-views-and-curves-of-the-same-stored-values.md)) are `ProcessingDefinition::preset_rows`, so the two never share a name.
- **A tool's place** ([0086](0086-import-as-a-full-window-sheet.md), planned), the side panel or Import, is a field of `ToolDefinition` when it is built, since every tool has one.
- **`ProcessingTool` stays in `apertrust-tools`,** run by the engine's session, with no egui ([0028](0028-tools-bound-to-operations.md)). **`UtilityTool` lives in the app,** since a utility's behaviour is its widgets: `apertrust-tools` keeps no egui, and the engine never sees a utility.
- **One `Context` for every utility:** the mirror, the settings, the words and a way to send commands, the frame's command list. History's steps, Browse's folder, Scopes' measurements and Export's jobs are in the mirror, and Export's choices in the settings, so one context covers them all. `body` is required; `reset` and every later method are provided as no-ops.
- **`UTILITIES` lists every utility's `UtilityDefinition`,** Browse among them, as `TOOLS` lists the processing tools.

### One header, one page drawer, one ⋯

- **The app draws every tool's header, page, ⋯ menu, open state and layout from `&ToolDefinition`,** so ↺, presets and ⋯ behave alike in every tool. This departs from darktable, which builds the header twice (`dt_iop_gui_set_expander`, `dt_lib_gui_get_expander`).
- **`apertrust-ui` keeps no tool types** ([0036](0036-the-design-system.md)): the app turns a `ToolDefinition` into the design system's `Section` and rows.
- **The header's buttons follow the declaration:** ↺ where `reset` is not none, presets where `presets` is true, ⋯ always, holding the tool's declared `MenuItem`s at `Place::Options` before the layout's items (decision 91). History's Compress History, Truncate History, Create a Style… and Discard History… become such declared items, four, ⋯'s most. Truncate History, built as Compress History's alternate while Option (Alt) is held (`history::options`), becomes an item of its own, so a declared item stays one action and `MenuEntry` needs no alternate.

### A utility's page

- **A utility uses the same `Row`s** as a processing tool. `UtilityDefinition::check` refuses every row that edits: a row naming a control (a slider, a choice, a toggle, `Row::When`, a control's row of buttons, `Row::Buttons`, a custom row naming controls), a preset row, a picker's rows (`Row::Pickers`, `Row::Patches`, `Row::Reference`, `Row::LastPick`) and a computed value. A utility's rows are settings, labels, groups, `Row::WhenSetting`, actions and buttons, each running a registry action, and custom rows naming no control. A utility's action rows and buttons name registry actions, each with its own availability, never a generated `tool-<tool>-<action>`, whose rule, a photo that takes edits, is a processing tool's ([0091](0091-context-menus-places-and-tools-items-as-actions.md)): Import's Use $(MONTH) is available while the sheet shows the problem it fixes, with or without a photo in the window.
- **All variation goes into custom widgets.** `Row::Custom { widget }`, as the tone equalizer's graph is today, also covers History's steps, Browse's recents and folder tree, the scope graph and Export's path line, each a `Widget` of the closed registry. The page drawer draws every other row itself and, reaching a utility's custom row, calls that utility's `UtilityTool::body` with the row's `Widget`, once per custom row: Browse's `body` is called for its recents line and again for its folder tree. Each utility matches its own widgets in `body`, whose egui code stays in its app module, takes that utility's state and returns its own actions. A processing tool's custom rows, as the tone equalizer's graph, stay drawn by the app's own `match` on `Widget`, as today.
- **A utility with presets has presets of its setting rows,** `presets` true in its `ToolDefinition`: Export for now, History, Scopes and Browse having none. They are presets of its setting rows, as darktable's `get_params` and `set_params` are presets of a utility module's state, with no binary blob. Export's are the first ([0090](0090-export-as-a-photos-tab-tool.md)).
- **A setting row may name its key in full.** Export's rows name `export.*` settings by their keys as strings, and the app binds them to `apertrust-export`'s settings; a test fails on a key that is not in the settings schema. `apertrust-tools` does not depend on `apertrust-export`, as it depends on no operation crate, and `apertrust-export` stays its own crate ([0056](0056-export-as-an-engine-job.md)).

| Tool | Its page |
| --- | --- |
| Exposure, AgX and the other processing tools | As now |
| Tone equalizer | As now, its graph a custom row |
| Export | Groups of setting rows (Storage options, Format options and the rest of [0090](0090-export-as-a-photos-tab-tool.md)), format-specific rows shown only for their formats (`Row::WhenSetting`), the path line a custom row, presets in the header |
| Scopes | A setting row for which scope, drawn as its buttons over the graph are today, the graph a custom row |
| History | The steps a custom row; ↺ the `reset` action; ⋯ holding Compress History, Truncate History, Create a Style… and Discard History… as declared items |
| Browse | The recents line a custom row; a Folders group holding the tree as a custom row |

### darktable's `lib_api.h`, and where each part goes

| darktable | What it does | Apertrust |
| --- | --- | --- |
| `name`, `description`, `expandable`, `has_preset_label` | Identity and header | `ToolDefinition` |
| `views`, `container`, `position` | Where it sits | The workspace's layout ([0083](0083-the-side-panels-layout-dragged-and-floating.md)) |
| `gui_init`, `gui_cleanup`, `gui_update` | Builds its widgets | `UtilityTool::body`, each custom row drawn each frame |
| `gui_reset` | The header's ↺ | `ToolDefinition::reset`, `UtilityTool::reset` |
| `get_params`, `set_params`, `legacy_params`, `init_presets`, `manage_presets`, `preset_autoapply` | Presets of its state | Presets of its setting rows |
| `set_preferences` | Items in its presets menu | Its ⋯ items ([0091](0091-context-menus-places-and-tools-items-as-actions.md)) |
| `gui_tool_box` | A widget in the top bar | Its toolbar items ([0092](0092-the-toolbar.md)) |
| `gui_post_expose`, `mouse_moved`, `button_pressed`, `scrolled` | Draws over the photo, takes the pointer | Later: `UtilityTool::overlay` and `pointer` |
| `view_enter`, `view_leave` | View changes | None: there is one view ([0037](0037-one-view-the-photos-tab.md)) |

### The steps

Each step is one pull request, reviewed before the next, but 93.2 and 93.3, which are one.

1. **93.1, the renames and the parent,** built: mechanical, with no behaviour change: `Definition` to `ProcessingDefinition`, `Utility` to `UtilityDefinition`, the trait `Tool` to `ProcessingTool` (about 250 uses), `Definition::presets` to `preset_rows`, and the shared fields into `ToolDefinition`, `Utility::menus` among them. The docs' names follow in the same pull request.
2. **93.2, `UtilityTool` in the app,** built, with History, Scopes and Browse implementing it, Browse joining `UTILITIES`, `main.rs` calling them through the trait. Until step 93.4 each utility's page is one custom row, a `Widget` of its own (`History`, `Scopes`, `Browse`), its `body` drawing what its module drew.
3. **93.3, one header and ⋯** for every tool in `TOOLS` and `UTILITIES`, built with 93.2: `ToolDefinition::reset` and `presets`, the header drawn from `&ToolDefinition`, History's ⋯ items declared with it, Truncate History an item of its own, so its ⋯ never goes empty; Export's comes with step 93.5, when it joins `UTILITIES`.
4. **93.4, the utilities on the page model:** Scopes, the smallest, first, then History, then Browse.
5. **93.5, Export's page as setting rows,** with its presets: Export implements `UtilityTool` and joins `UTILITIES`, its section drawn by `export.rs` until then.

## Options considered

- **Two models, as today:** each utility draws its own header and page, so ↺, presets and ⋯ drift apart, Browse is in no list, and Export's setting rows, presets and the Import sheet's tools, all settings, would each be drawn by code of their own.
- **One flat definition for both,** a utility leaving the editing fields empty: a utility would carry a version, layers, Auto, switched, controls and pickers that mean nothing for it, each needing a check that it is empty; darktable keeps the two apart.
- **One trait for both kinds,** in `apertrust-tools`: a utility's behaviour is its widgets, which needs egui there, against [0028](0028-tools-bound-to-operations.md); the processing half is what the engine runs, which a utility never is.
- **A shared header only,** each utility's page still written by hand: the header would agree, but setting rows, their presets and a check on what a utility may show would not be shared, and Export's and Import's rows would stay two codes.
- **The header built per kind, as darktable's two expanders:** two codes to keep alike for one block that looks the same.
- **`apertrust-tools` depending on `apertrust-export`,** so Export's rows name typed settings: it couples the tool layer to one utility's crate and its writers; keys as strings with a test against the schema keep the boundary and fail as early.
- **Truncate History kept as Compress History's alternate,** shown while Option is held: a declared item would need to name a second action, a field of `MenuEntry` for one item, and the alternate is hidden from anyone who doesn't know to hold Option.
- **Keeping "section" for utilities,** or "utility" as a level beside tools: two names for what the side panel shows, moves and floats alike, rejected already in [0004](0004-tools-for-people-operations-for-algorithms.md).

## Consequences

- **[0004](0004-tools-for-people-operations-for-algorithms.md)** names the parent and its two kinds; **[0028](0028-tools-bound-to-operations.md)** describes `ProcessingDefinition` and `ProcessingTool` over the shared parent; **[0086](0086-import-as-a-full-window-sheet.md)** puts a tool's place in `ToolDefinition`; **[0090](0090-export-as-a-photos-tab-tool.md)** declares Export's page as a utility's setting rows; **[0091](0091-context-menus-places-and-tools-items-as-actions.md)** and **[0092](0092-the-toolbar.md)** declare every tool's items in `ToolDefinition::menus` and `ToolDefinition::toolbar`.
- `apertrust-tools` holds `ToolDefinition`, `ProcessingDefinition`, `UtilityDefinition`, `ProcessingTool`, `UTILITIES` with every utility and `UtilityDefinition::check`; `Widget` gains the utilities' widgets. The app holds `UtilityTool`, its `Context`, the one header and page drawer, and each utility's widgets in its module. No crate is added, and no dependency changes.
- Nothing a person sees changes but one item, and no setting, no Fluent id, no session command and nothing in the sidecar: each utility keeps its look on the page model, its custom widgets drawing what its module draws today. The one is History's ⋯, where Truncate History, today Compress History's alternate while Option is held, becomes an item of its own in step 93.3, with the words it has. A change of a utility's look is a design question of its own. The engine and its session are untouched but for the trait's name.
- The new names are built, step 93.1, and `UtilityTool` with the one header and ⋯, steps 93.2 and 93.3; the utilities' pages as rows and Export's are planned. [gui.md](../gui.md#one-tool-model) describes the model, [the roadmap](../roadmap.md) holds the steps.
- The agent kit's `tool-bindings`, `ui-conventions` and `desktop-architecture` skills are to state the terms and the model.
