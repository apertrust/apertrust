# 0065. Names and crates

## Context

What is the project called in text, code and files, how are its crates named and laid out, and when does code get a crate of its own? Many small crates each add build time, a manifest and a dependency edge to keep, while a crate that protects no boundary protects nothing.

## Decision

**The name:** Apertrust, with a capital A where people read it, and `apertrust` in code: the crates `apertrust-<name>` and `apertrust-op-<id>`, the desktop binary `apertrust` and the CLI `apertrust-cli` ([0006](0006-two-programs-on-three-desktop-platforms.md)), environment variables `APERTRUST_*` (`APERTRUST_TEST_IMAGES`, `APERTRUST_USER_DATA_DIR`, `APERTRUST_VERSION`), the edit sidecars `.apxmp` and `.apdata` ([0048](0048-the-apxmp-format.md), [0049](0049-the-apdata-file-as-darktables-dtdata.md)), the library files `.aplibrary` and `.approject`, and the domain apertrust.org ([0067](0067-going-public.md)).

**The layout:** each crate lives in `crates/<name>`, its package name without `apertrust-`: `crates/engine`, `cli`, `raw`, `plugin`, `color`, `base`, `golden`, `display`, `operations`, `modules`, `export`, `library`, `ui`. The desktop app, package `apertrust`, lives in `crates/app`. Operation crates, `apertrust-op-<id>`, live in `crates/op/<id>` ([0020](0020-operations-as-native-plugins-one-crate-each.md)), which leaves `crates/operations` to the list crate. The workspace lists the top-level crates by name, since Cargo's globs cannot leave `crates/op` out of `crates/*`, and keeps a glob for `crates/op/*`; a new crate goes into `members` by name. Shared dependency versions live under `[workspace.dependencies]`.

**Few crates.** New code goes into a module of an existing crate. A crate of its own only for:

- `unsafe` FFI: `unsafe` is denied workspace-wide, and only an FFI crate opts out with `#![allow(unsafe_code)]` at its root (`apertrust-display`);
- a dependency direction that would otherwise break;
- a decided boundary, such as an operation's.

Kept apart, each for its reason: `apertrust-display` (FFI), `apertrust-raw` (rawler), `apertrust-color` (the kernels' Slang import path), `apertrust-plugin`, `apertrust-modules` and `apertrust-operations` (layer boundaries), `apertrust-export` (the writers without the GPU), `apertrust-library` (the library without the GPU, which the engine depends on), `apertrust-ui` (egui) and the operation crates. The sidecar's text layer is `apertrust_library::sidecar`, the safe write `apertrust_base::fs`. `apertrust-fs` is made only if bulk listing needs FFI that `rustix` and the `windows` crate do not cover safely ([0053](0053-finding-folders-and-photos.md)). Planned crates (`apertrust-protocol`, `apertrust-server`, `apertrust-mcp`) are weighed against this rule when their phase comes. A crate is added when its first real code lands, never before.

**Shared helpers move at the second port.** A helper ported from darktable's `src/common/` stays in its port's crate until a second port that uses it is scheduled; then it joins an existing shared crate, `apertrust-color` for image maths or `apertrust-plugin` for the pass interface, written for both users from the start and naming the scheduled port in its documentation; if that port is dropped, it moves back. A new shared crate waits for a second port that uses it. So `apertrust-color` holds colour maths and the numerical helpers ports share (curve interpolation, box means, Gaussians).

## Options considered

- The name written ApertRust or apertrust in prose.
- Operation crates nested inside the list crate's folder: one package's folder holding others.
- Renaming the list crate for a folder's sake.
- A crate per concern from the start (a sidecar crate, a file-system crate, a jobs crate, a settings crate, a translation crate): boundaries with one user each.
- Sharing every helper from the start, or moving one back and forth between ports.

## Consequences

- A proposal for a new crate names which of the three reasons applies.
- AGENTS.md's crate table lists every crate with its role; [crates.md](../crates.md) holds each crate's state, built and planned.
