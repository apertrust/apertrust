# 0033. One tone mapper per edit

## Context

darktable offers several tone mappers (AgX, sigmoid, filmic rgb), each a module, and lets an edit stack them. A tone mapper compresses the scene's range for display; run twice on the same pixel, it compresses twice. How does an edit choose its tone mapper, and how do layers and other modules meet it?

## Decision

**Tone mappers are modules of their own, as in darktable:** AgX now, Sigmoid and Filmic RGB as their modules come. They sit in the Base tab, where darktable's scene-referred workflow has its tone mapper ([0038](0038-side-panel-tabs-as-darktables-groups.md)).

**The background has exactly one tone mapper, chosen per edit:**

- **A setting chooses the tone mapper for new photos** and which module the Base tab shows.
- **Each edit names its tone mapper's module and keeps it;** an edit that names none is AgX's. Exactly one tone mapper operation is planned for the background, its parameters global.
- **A photo using another tone mapper shows that module,** with a note.
- **Switching an existing photo is an explicit action,** one undo step; the setting never changes the tone mapper of edits already made.

**A layer may choose its own tone mapper,** planned with layers ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)): any of the three, which replaces the background's within the layer's mask, never stacked on it, so a pixel is tone-mapped once. darktable's masked tone-mapper instances import onto such layers ([0051](0051-importing-darktables-edits.md)).

**Every other module a layer can use acts at its darktable place** ([0022](0022-pipeline-order-and-the-default-pipeline.md)): those before the tone mapper shape the light it compresses, so a slider means the same on a layer as on the background, and darktable's display-referred effects (grain, vignetting and the others after it) follow it.

## Options considered

- One fixed AgX with sigmoid-like looks: not darktable's modules, and an edit made with sigmoid or filmic would not carry over.
- A tone mapper's parameters fitted from other modules' controls: a reducing control that is not darktable's.
- The setting changing the tone mapper of edits already made: an edit would change its look.
- Writing the tone mapper module into every edit even at its defaults: an edit naming none is already AgX's.
- A layer's tone mapper stacked on the background's: a pixel tone-mapped twice.
- Tone mappers on the background only: darktable's masked tone-mapper instances would not import.

## Consequences

- Sigmoid and filmic rgb's operations are ported; their modules wait for an edit to be able to choose them.
- The form of the one-tone-mapper rule in the edit is settled with Sigmoid's module.
