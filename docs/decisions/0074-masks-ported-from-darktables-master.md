# 0074. Masks ported from darktable's master

## Context

Layers' masks follow darktable's Flexi masks ([0034](0034-layers-masked-module-instances-in-a-fixed-order.md)). Flexi is on darktable's master, 5.7.0-dev, and in no stable release: the reference release, 5.6.2, has darktable's classic masks ([0003](0003-the-reference-is-darktables-latest-stable-release.md)). Its formats look settled while its panel and preset behaviour still change. Masks and retouch come in Phase 4, after the migration and its reference rule end ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Decision

- **Masks are ported from darktable's latest master,** at a commit the port spec names, `docs/ports/masks.md` (planned): Flexi's model, not its code or panel. That is its elements, its groups folded in list order, its refinement, and its rules for a missing mask, as decision 34 uses them.
- **The spec follows master's later changes to the mask code** with the port-module skill's steps for an upstream change, until a stable darktable release holds Flexi. That release is then the masks' reference.
- **Retouch is ported the same way,** with layers in Phase 4 (decision 34): darktable's retouch module, heal and clone, from the same master commit.
- **Their golden cases render with a darktable-cli built from that commit,** its version kept apart from `REFERENCE_VERSION`, so they never mix with the migration's references (planned).

## Options considered

- **Wait for the stable release that holds Flexi:** the spec and the spikes would wait, and the port would follow an unreleased design anyway once the release came.
- **darktable 5.6.2's classic masks:** a mask per instance, read on that instance's own pixels, which layers do not have; and the model darktable is leaving.

## Consequences

- `docs/ports/masks.md` names the master commit in its header, as other ports name the reference release.
- The golden harness gains a second darktable-cli for the masks' cases (planned with Phase 4).
- Decision 3's rule is unchanged for the migration's modules.
