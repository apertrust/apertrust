# 0004. Tools for people, operations for algorithms

## Context

Apertrust ports darktable's processing, and darktable calls both what a person works with and the algorithm behind it a "module". Rust, Slang and wgpu each have a "module" of their own too. One word with several meanings in one codebase confuses people and code alike. What are the two levels called, who owns a parameter, and can a person reach an algorithm directly?

## Decision

**Two terms.**

- A **tool** is what a person sees and works with: one section of the side panel, with its controls, tabs and collapsible groups, under one name ([0028](0028-tools-bound-to-operations.md)).
  - **Everything in the side panel is a tool:** the tools that edit the photo, such as Exposure or Colour calibration, and Browse, Filters, Keywords, Metadata, Scopes, History, Snapshots and the Colour picker.
  - **A processing tool** binds operations, as darktable's processing modules do.
  - **A utility** is a tool that edits nothing, such as Scopes or the Colour picker, as darktable's utility modules. "Utility" names only that kind of tool, never a second level beside tools.
  - **A tool has controls and settings.** Its controls are what an edit stores. Its settings are the options darktable keeps in darktablerc, which Apertrust keeps in `settings.toml`, never in an edit.
- An **operation** is one algorithm in the engine, usually a darktable port, with its manifest, schema and kernels ([0020](0020-operations-as-native-plugins-one-crate-each.md)). darktable's own XMP history calls these operations (`darktable:operation`).
- **An MCP tool is always written "MCP tool",** never a bare "tool". The MCP tools are generated from the tools' definitions and the library's commands ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

**"Module" names only darktable's own things and the languages' own modules.**

- **darktable's:** a darktable module, its source (`src/iop`, `src/libs`) and its identifiers (`dt_iop_module_t`, `dt_module_introspection`); its module groups and its utility modules; its strings and data files quoted verbatim; an operation manifest's `darktable_module` provenance field; the `port-module` skill, `/port <module>`, the port spec's template and the "Port a darktable module" issue form.
- **The languages':** Rust's `mod` and "this module" in its doc comments; Slang's `module` declarations; wgpu's and naga's `ShaderModule`, `naga::Module` and `ModuleInfo`.
- **Porting a darktable module yields an operation and a tool.**

**Every operation parameter has exactly one owning tool.** A test refuses a parameter owned twice. A parameter with no editing meaning and no darktable widget is fixed at darktable's value, owned by its tool with no control.

**Operations never appear in the interface.** There is no Expert tab or direct operation access. Depth comes from inside a tool, its own tabs and collapsible groups ([0029](0029-a-tool-is-darktables-page.md)), never from a second tool. A read-only operation inspector in debug builds may show what each tool set.

**Tools sit behind the session protocol.** Clients send tool values and the session turns them into operation parameters, so every client, the app, scripts, MCP and a remote interface, sees the same tools ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

## Options considered

- **Keeping darktable's "module" for what a person works with:** darktable's users know it, but it is darktable's word for both levels and the languages' word for a unit of code, so every sentence would need a qualifier. Photo editors, commercial raw editors among them, call these tools, and the interface speaks the person's language.
- **Keeping "tool" for MCP alone:** an MCP tool is a protocol's entry point, generated from a tool's commands. Writing "MCP tool" in full keeps the two apart at the cost of one word.
- **"Adjustments":** Browse, Scopes and History adjust nothing.
- **"Panels":** the side panel is a panel, and a floating panel holds a tool ([0083](0083-the-side-panels-layout-dragged-and-floating.md)); a tool would be a panel inside a panel.
- **Tools for editing and "sections" for the rest:** two names for what the side panel shows, moves and floats alike ([0083](0083-the-side-panels-layout-dragged-and-floating.md)). "Section" stays the side panel's word for the place a tool is shown.
- **Reading the sidecar's old names beside the new:** a compatibility path kept for good, for files only nightly builds wrote.
- **Keeping `apertrust:module` in the sidecar:** free, but the file would disagree with the vocabulary for good.
- **An Expert tab with direct operation access:** a second, unowned way to set parameters tools drive; operation parameters in saved edits beside tool values; ports no tool needs; darktable's whole surface back in the interface.

## Consequences

- **The docs, code and skills use the two terms.** Until the rename's step for each lands, the code keeps its names: the crate `apertrust-modules` in `crates/modules`, the `Module` trait and its types, the Fluent ids `module-<id>-…` and the sidecar's `apertrust:module`.
- **The rename lands in steps,** each planned, the identifiers in backticks following their own step:
  - 4.1: a tool's settings named settings in the code, freeing the word;
  - 4.2: the docs say tool, `docs/modules/` moved to `docs/tools/`;
  - 4.3: `apertrust-modules` renamed `apertrust-tools` in `crates/tools`, with the trait, its types, the session's commands and events, and the CLI's help and errors;
  - 4.4: the code's own names, tests and comments;
  - 4.5: the Fluent ids `module-…` renamed `tool-…`, in `tools.ftl`;
  - 4.6: the words a person sees and hears, in every language: "tool", "інструмент" in Ukrainian, "narzędzie" in Polish, including the tooltips quoted from darktable, whose wording otherwise stays darktable's;
  - 4.7: the sidecar's `apertrust:tool` and `apertrust:toolVersion` in place of `apertrust:module` and `apertrust:moduleVersion` ([0048](0048-the-apxmp-format.md));
  - 4.8: the agent kit's skills, agents and commands.
- **The sidecar keeps no reading of the old names.** Only nightly builds have written them, never a release, so after step 4.7 a file a nightly wrote opens view-only, its field unknown, until Discard History… clears its edit ([0078](0078-undo-redo-and-the-edits-history.md)). That cost is accepted while Apertrust is in early development.
- **What people and files hold keeps its meaning.** Tool ids (`exposure`, `toneequal`) are permanent ([0028](0028-tools-bound-to-operations.md)); `settings.toml`'s keys are unchanged, a tool's settings already under `tools.<id>.<setting>`; the CLI's flags are unchanged, `--set` and `--reset` taking a tool id and a control; a message id the sidecar stores is never renamed.
- **The `tools/` scripts folder, the `tool-bindings` skill and `ToolButton` keep their names.** The skill already means the tools of this decision; the others are a folder and a toolbar's button.
- An edit stores tool values, and the bindings derive operation parameters from them.
- `--param` and `Command::Set` reach only a parameter no tool owns yet, of an operation an unedited photo runs; a fixed parameter is refused (`EditError::Fixed`).
