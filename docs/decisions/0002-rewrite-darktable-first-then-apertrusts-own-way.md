# 0002. Rewrite darktable first, then Apertrust's own way

## Context

A major part of Apertrust is built on darktable's trusted colour science. How close should the first modules stay to darktable, when does that closeness end, and what is Apertrust's relation to darktable afterwards? Mirroring darktable needs no product choice per control, since every darktable widget is already a view of one stored parameter, and it proves the engine and the edit on modules photographers know.

## Decision

**The first pass rewrites darktable's modules as darktable has them,** in Apertrust's interface: one module per darktable module Apertrust includes, under darktable's name, its parameters as the controls under darktable's labels and ranges ([0029](0029-a-module-is-darktables-page.md), [0030](0030-controls-as-darktables.md)).

**The mirroring rule:**

- darktable's names, in British spelling.
- A first-pass module's id is its operation's (`exposure`, `toneequal`, `colorbalancergb`, `temperature`).
- The raw modules are darktable's three, Raw black/white point, Highlight reconstruction and Demosaic, not one combined module.
- Both of darktable's white balances, as darktable has them ([0027](0027-both-of-darktables-white-balances.md)).
- Modules that do another's job are left out: colour zones (the colour equalizer's), monochrome (colour calibration's grey), base curve and the other display-referred legacy tone tools, and every module darktable marks deprecated.
- darktable's processing as darktable has it: its v5.0 pipeline order ([0022](0022-pipeline-order-and-the-default-pipeline.md)), its parameters and defaults (colour balance rgb's `contrast` as darktable's percentage and `grey_fulcrum` at 18.45 %, darktable UCS its default saturation formula), the tone equalizer's own curve, RGB curve drawn as darktable's graph.
- Whatever follows from the rule is decided by the rule. Only a deviation, scope darktable does not cover (layers and masks, the library, export, AI) and blockers need the maintainer's decision.

**The agreed differences**, kept because they make the later evolution easier: one view and Apertrust's design system; layers in place of darktable's instances and per-module masks ([0034](0034-layers-masked-module-instances-in-a-fixed-order.md)); darktable's order kept fixed; tone mappers as modules of their own, one per edit ([0033](0033-one-tone-mapper-per-edit.md)); darktable's legacy and overlapping modules left out.

**The migration ends once, for the whole project,** at the end of Phase 3, when:

- every first-pass module is in the app and passes its comparisons, and
- darktable's edits that use only ported modules import and render as in darktable ([0051](0051-importing-darktables-edits.md)).

**Until then** image quality is judged against darktable-cli at the reference release ([0003](0003-the-reference-is-darktables-latest-stable-release.md)): golden ΔE2000 comparisons with per-case tolerances ([0061](0061-golden-image-tests-against-darktable-cli.md)) and the whole-pipeline comparison ([0062](0062-the-whole-pipeline-comparison.md)).

**After it** Apertrust evolves on its own:

- Its own renders, taken at the commit that completes the migration, are the baselines. A change to a baseline is a deliberate, reviewed decision with its size stated, never a re-render to make a check pass.
- The slider tables held to darktable's (`DarktableSlider`, `testing::slider_differences`) become Apertrust's own recorded slider baselines (`SliderSpec`): a slider change is then a reviewed baseline change, as a render's is.
- It claims no dependency on darktable: neither the harness's gates nor the docs present darktable as the reference.
- The rule for departures during the migration ([0063](0063-departures-from-darktable-and-views-below-full-scale.md)) ends: a processing change is a baseline change.

**At every stage:** nothing of darktable is ever loaded or linked; edits made with the migrated modules keep looking exactly as they did; files ported from darktable keep darktable's copyright and provenance, as the GPL requires ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)).

## Options considered

- Simplified modules of Apertrust's own design before the rewrite works: one product choice per control that sets several parameters, before the engine and the edit are proven.
- Mirroring every darktable module, legacy and overlapping ones included: copies the confusion the agreed differences remove.
- darktable's modules without a module layer, operations and their parameters in the edit: makes any later change of modules a migration of the edit format.
- A per-module end of the migration: modules share colour maths and a pipeline, so one module evolving while its neighbours are compared would make the whole-pipeline comparison meaningless.
- darktable as the permanent reference: ties every later improvement to darktable's behaviour.
- Ending the comparisons at once: the migration is what earns the trust; without darktable's renders a port's error has nothing to show it.

## Consequences

- Modules stay a layer over operations, and edits store module values with versions ([0028](0028-modules-bound-to-operations.md)), so later modules can rebind to the same operations and old edits migrate without touching a kernel.
- [modules/README.md](../modules/README.md) holds the first pass's modules and rules; each port follows darktable's source at the reference release.
- Before the end of Phase 3: where Apertrust's own baselines are kept (per GPU), and what the cases' inputs become.
