# 0100. Groups: plain and collapsible, one parent

## Context

darktable titles a module's rows in two ways: a section label (`dt_ui_section_label_new`), a title over the rows after it, and a collapsible section (`dt_gui_new_collapsible_section`), a title with a chevron that hides its rows ([0029](0029-a-tool-is-darktables-page.md)). How should a definition hold them? Which rows a title covers has to be stated, not implied by their order; screen readers should hear a group named by its title either way; a rule about titled rows should be written once for both; and a tool's first row should sit the same distance under its header whatever it is, a titled one included, as the Export tool's Storage options.

## Decision

**A group is one parent with two kinds,** as a tool is a processing tool or a utility ([0093](0093-one-tool-model-processing-tools-and-utility-tools.md)):

```rust
/// A titled set of rows inside a tool's tab.
Row::Group(Group { id: &'static str, kind: GroupKind, rows: &'static [Row] })
pub enum GroupKind { Plain, Collapsible }
```

| Kind | Is | darktable's | Drawn |
| --- | --- | --- | --- |
| **Plain group** | A title over its rows, always open, with no chevron: Export's Storage options and Format options, White balance's Scene illuminant temp, Colour calibration's Target | section label | `PlainGroup` |
| **Collapsible group** | A title with a chevron, closed at first but where a utility declares it open, as the Export tool's Global options ([0090](0090-export-as-a-photos-tab-tool.md)), its open state the app's, never the edit's ([0029](0029-a-tool-is-darktables-page.md)) | collapsible section | `CollapsibleGroup` |

- **Both kinds hold their rows,** so changing a group's kind is a one-word change to its `kind`, its title's message then the new kind's (below). There is no label row: the rows under a section label, up to the next group or the end of its container, are its plain group's, and rows before a container's first group stay outside any group, as in darktable.
- **Nesting:** a plain group may sit inside a collapsible one, as Colour calibration's Target inside its area colour mapping, and later Export's General settings and Per metadata settings inside Metadata. A collapsible group never sits inside another group, and a plain group never inside a plain one. `ProcessingDefinition::check` and `UtilityDefinition::check` refuse any other nesting (`DefinitionError::GroupInGroup`) and a plain group with no rows (`DefinitionError::EmptyGroup`); a collapsible group with no rows keeps its meaning, nothing behind it yet, drawn disabled and closed.
- **Ids:** a group's id is unique among the tool's groups of both kinds and its rows of pickers' labels (`DefinitionError::DuplicateGroup`). Its title's message follows its id and kind: `tool-<tool>-group-<group>` for a plain group, `tool-<tool>-collapsible-<group>` for a collapsible one (`messages::group_title`), unless a utility declares its own. Collapsible groups keep the ids their open state is kept by.
- **Drawing,** in `apertrust-ui`: both kinds share one title type, `group_label_text`'s caption-sized Medium capitals, and, to screen readers, are a group (`Role::Group`) named by the title with its rows inside it, one helper drawing that group for both. A plain group's title is no node of its own; a collapsible group's row stays its button with `expanded` and "N set".
- **The first-row rule:** a group that is the first row of its container, a tool's body, a tab or a group, draws no space above itself; elsewhere a plain group keeps 8 points above it (`spacing.m`) and a collapsible group 4 (`spacing.xs`). The gap above a tool's first row is so the same in every tool, whatever the row: 4 points under its header, or 8 under its tab strip. The container tells its first row, never the tool: the app's one page drawer passes `first` to `PlainGroup` and `CollapsibleGroup` when nothing is drawn yet in the container, rows the photo hides or a mode leaves out not counted.
- **"Group label"** now means a plain group's title.

## Options considered

- Two rows kept, the label given a list of the rows it titles: still two kinds of row, every rule written twice.
- Every group collapsible, a plain group one that cannot close: a chevron and a focus stop on a title that does nothing.
- One message id for both kinds' titles: a change of kind with no change of message, but every collapsible group's message renamed in every language.
- The first-row rule in each tool, a flag in its definition: the gap then depends on each tool remembering it.

## Consequences

- `apertrust-tools`: `Group` and `GroupKind`; every tool and utility page converted with its rows in the same order, each control still in exactly one row; the checks for nesting, empty plain groups and ids.
- `apertrust-ui`: `PlainGroup` replaces `group_label`; `CollapsibleGroup::first`.
- The Export tool's first row starts 4 points under its header, as every tool's does.
- [gui.md](../gui.md#tool-panels-from-definitions) describes the groups, their spacing and the first-row rule; [crates.md](../crates.md) the types and components.
