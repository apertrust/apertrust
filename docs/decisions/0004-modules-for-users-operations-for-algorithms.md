# 0004. Modules for users, operations for algorithms

## Context

Apertrust ports darktable's processing, and darktable calls both what the user sees and the algorithm behind it a "module". Two meanings of one word in one codebase confuse people and code alike. Who owns a parameter, and can a user reach an algorithm directly?

## Decision

**Two terms.**

- A **module** is what the user sees and edits with: its controls, tabs and collapsible groups, under one name ([0028](0028-modules-bound-to-operations.md)).
- An **operation** is one algorithm in the engine, usually a darktable port, with its manifest, schema and kernels ([0020](0020-operations-as-native-plugins-one-crate-each.md)). darktable's own XMP history calls these operations (`darktable:operation`).
- "Tool" is not a user-level term; it keeps only its MCP meaning.

**Every operation parameter has exactly one owning module.** A test refuses a parameter owned twice. A parameter with no editing meaning and no darktable widget is fixed at darktable's value, owned by its module with no control.

**Operations never appear in the interface.** There is no Expert tab or direct operation access. Depth comes from inside a module, its own tabs and collapsible groups ([0029](0029-a-module-is-darktables-page.md)), never from a second module. A read-only operation inspector in debug builds may show what each module set.

**Modules sit behind the session protocol.** Clients send module values and the session turns them into operation parameters, so every client, the app, scripts, MCP and a remote interface, sees the same modules ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

## Options considered

- Keeping "module" for both levels, with "tool" for the user's: two words that each meant something else elsewhere.
- An Expert tab with direct operation access: a second, unowned way to set parameters modules drive; operation parameters in saved edits beside module values; ports no module needs; darktable's whole surface back in the interface.

## Consequences

- The docs, code and skills use the two terms: `apertrust-modules` for modules, `crates/op/<id>` for operations.
- An edit stores module values, and the bindings derive operation parameters from them.
- `--param` and `Command::Set` reach only a parameter no module owns yet, of an operation an unedited photo runs; a fixed parameter is refused (`EditError::Fixed`).
