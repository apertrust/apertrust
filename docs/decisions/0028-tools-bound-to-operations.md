# 0028. Tools bound to operations

## Context

A processing tool is what a person edits with; an operation is the engine's algorithm ([0004](0004-tools-for-people-operations-for-algorithms.md)). Where do tools live, how does a control become operation parameters, which commands and events carry tool values, and how do edits store them, so that every client sees the same tools, an untouched tool changes nothing exactly, and later tools can rebind the same operations?

## Decision

**Static tool definitions and pure bindings in `apertrust-tools`, run by the engine's session.**

**The crate,** `crates/tools`, depends on `apertrust-plugin`, `apertrust-color` and `apertrust-base`'s settings part, for the generated statics of its tools' settings (decision 43), only: no egui, wgpu, engine or operation crate; bindings name operations and parameters by id. The engine depends on it, since the edit, history and sidecar it owns hold tool values, and `Session::start` takes the list the binaries hand it (`TOOLS`), as with operations.

**A definition** is static data: an id (snake_case, permanent), a version, whether it works on layers, whether it has Auto, its controls and its tabs. A **control** is what an edit stores: id, kind (a number with range, step and default; a choice; a boolean; curve points) and unit. A **row** is how a tab shows a control: a slider, a choice, a toggle, a group label, a preset row, a view's coordinate, or a custom widget from a closed registry naming the controls it edits ([0029](0029-a-tool-is-darktables-page.md)). Message ids follow from the ids (`module-<module>-title`, `module-<module>-control-<control>-label`, `-option-<option>`, `-tab-<tab>`, `-group-<group>`; units `unit-<unit>`), in `crates/ui/locales/<locale>/modules.ftl`, checked by a test.

**A binding** is `Tool::bind(values, image, base)`: a pure function of the tool's own values, the photo's image context at full resolution and `base`, each operation's `Operation::defaults(image)`, returning the operation parameters to set. It never sees a pipe, so the viewport and an export get the same parameters, nor another tool's values, so the order in which controls were used cannot matter. `defaults(image)` gives the defaults that come from the photo; `owns` the parameters it owns; `migrate` older versions. Transfer functions are named pure functions with their derivatives, tested: monotone where promised, nothing set at the default, the ends inside the schema, the derivative against finite differences. The trait's other provided methods are [0030](0030-controls-as-darktables.md)'s and [0031](0031-views-and-curves-of-the-same-stored-values.md)'s.

**Untouched is exact.** An operation no unedited photo runs is planned only while a binding sets one of its parameters, or while its switched tool is on ([0032](0032-tools-off-until-first-touched.md)). A tool at its defaults sets nothing, so an untouched tool leaves the plan, its keys and its frames unchanged. One function, `edit::instances`, turns an edit into instances for the session, the export, the CLI and the tests.

**Ownership:** each tool declares what it owns; a test refuses a parameter owned twice, unknown or of the wrong kind, and a binding that sets anything else. At run time a misbehaving tool is reported and left out rather than failing the photo.

**Commands:** `SetControl { tool, control, value, done }`, where `done` ends a gesture and setting a control keeps it; `ResetControl`; `ResetTool`; `Keep { tool, control, kept }`; `ApplyPreset`; `Reset`; and `Set` only for a parameter no tool owns, of an operation an unedited photo runs, so a saved value can never wait unseen on an optional operation. The values report carries each tool's controls with their defaults for the photo, their marks (kept, Auto), the unowned parameters, `edited` and `through`; `Saved` follows each write.

**Edits:** `.apxmp` version 1 holds each tool touched, with its version, the controls set (curve points as `x,y` pairs), its kept and Auto marks and its on state, and operation parameters only where no tool owns them, on operations an unedited photo runs; others are dropped with a warning on reading. The session saves after each completed change, and a change never marked done when the photo closes; a writer thread off the engine thread writes the latest edit, so a slow disk never delays a frame, and a failed write is reported while the edit stays in memory. Tool versions and migrations keep old edits' look; a tool neither known nor in `apertrust_tools::RETIRED` opens the photo view-only ([0048](0048-the-apxmp-format.md)).

**Invariants:** each tool has a table in `docs/tools/<id>.md`, one row per invariant, each a test in `crates/tools/tests` on a synthetic ramp on CI's software GPUs and on a corpus raw locally. Every table has a byte-identical row at the defaults.

**The product choices** the layer rests on:

1. Edits save themselves after each completed change; there is no Save command.
2. Setting a control keeps it, and the marker releases it, keeping its value.
3. Kept and Auto marks are saved with the edit.
4. A tool that is always on adds an optional operation only once it leaves its defaults. Where the operation at its defaults is not an identity (RGB curve's channel modes, colour balance rgb's gamut mapping), that first step can change pixels slightly, as darktable's first step does; each such tool's invariants measure it (Colour balance RGB's B1).
5. A control whose unit has no natural zero (temperature in kelvin) has its default, as shot, as its untouched value.
6. The zero invariant, and a switched tool's off invariant, are byte-exact, not within a tolerance.
7. Version 0 edits lose their owned parameters on opening, with a warning.
8. The CLI's `--set` and `--reset` take `TOOL.CONTROL`, and `--param` and `--reset-param` an `OP.PARAM` no tool owns.

## Options considered

- The tool interface in `apertrust-plugin`: mixes the two levels and protects no boundary.
- A tool layer wrapping the session: splits the edit from the engine.
- Hand-written panels, or declarative-only bindings: a second description of each tool, or a closed language that cannot state darktable's rules.
- Every operation always planned with an identity setting or an `enabled` parameter: colour calibration has no identity, and RGB curve's and RGB levels' defaults clip.
- A tolerance at zero, or one generic `SetValues` command.
- Derived operation parameters stored in the edit beside tool values.

## Consequences

- A tool that changes nothing changes no byte of the render.
- Open: how Auto reads image statistics ([0019](0019-scopes-and-readouts-counted-on-the-gpu.md)); a layer's `base` ([0034](0034-layers-masked-tool-instances-in-a-fixed-order.md)); a description of the tools for remote clients, which cannot call a tool's methods.
