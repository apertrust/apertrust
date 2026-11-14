# 0051. Importing darktable's edits

## Context

Many of Apertrust's first users come from darktable with years of edited photos. The first pass rewrites darktable's modules as darktable has them ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), each port already decoding darktable's parameters for its golden cases, so darktable's history can be read into the same modules. A partial import, though, would render a different photo from the one the person edited.

## Decision

**Apertrust imports darktable's edits** from darktable's `.xmp` sidecars, through the XMP layer ([0050](0050-our-own-xmp-layer.md)).

- **A module's values are read from darktable's history as darktable stores them,** its parameters decoded at their version as the port's spec describes them and set on Apertrust's module of the same id, so the photo renders as it does in darktable.
- **An edit that uses a module not yet ported is not imported.** Apertrust says which module is missing, and the photo opens as an unedited one would.
- **Ratings, colour labels and keywords** are read either way, since they are standard XMP.
- **Open, to settle when the import is designed:** how darktable's masks (its drawn and parametric masks, which darktable evaluates per module on that module's input), extra instances and custom module order map onto Apertrust's layers ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)), which evaluate range masks once per layer and keep a fixed order.
- **Planned.** The import is part of the migration's end ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)): when every first-pass module is in the app and passes its comparisons, a darktable edit of those modules imports whole.

## Options considered

- No import, only ratings, labels and keywords: a darktable user's past work would not carry over, though the ports decode darktable's parameters anyway.
- A partial import, leaving unported modules out: the photo would render unlike the one the person edited, with nothing to say so.
- Writing darktable's edits back: darktable's binary parameter blobs and history make it close to impossible, and nothing needs it.

## Consequences

- The ports' decoders of darktable's parameters, today written for the golden cases (`legacy_params` where a setup has one), are needed at every version darktable can store, each tested against darktable's own values.
- The import's design waits for layers, so masked edits are decided with them.
- The Library and folder mode show darktable's ratings, labels and keywords from the start.
