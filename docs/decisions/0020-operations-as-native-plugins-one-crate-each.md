# 0020. Operations as native plugins, one crate each

## Context

darktable's processing modules (iops) are each one unit: parameters, processing and interface together. How are Apertrust's operations packaged, how do they reach the engine, and can darktable's own modules or third parties' plug in? A kernel split from its operation spreads one algorithm over two places; darktable's code is C and is never to be loaded ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Decision

**Every operation is a native plugin behind one interface,** `apertrust-plugin`'s `Operation` trait ([0021](0021-the-operation-interface.md)), and **one crate**, `apertrust-op-<id>` in `crates/op/<id>`, holding what darktable's iop holds apart from its interface:

- its Slang source, one entry point per pass plus the derivative entry points ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md)), and the generated WGSL;
- its CPU-side setup, `prepare` (darktable's `commit_params`);
- its manifest and its parameter schema with versions;
- its provenance: the darktable module and the reference release it was ported from ([0003](0003-the-reference-is-darktables-latest-stable-release.md)).

**The trait is plain data plus `prepare`, without wgpu**, so declarative packages can implement it later.

**Built-in operations are compiled-in crates.** `apertrust-operations` lists them, `OPERATIONS` in pipeline order with their `KERNELS`. The engine depends only on `apertrust-plugin`, and the binaries hand it the list.

**The engine owns the kernels that belong to no operation:** the viewport's downscale, the region copy, the export's resample, the scopes, and from Phase 2 the fused kernels built from operations' per-pixel functions.

**Shared colour maths** lives in `apertrust-color`: Slang for kernels, Rust for setups. A helper from darktable's `src/common/` moves there once its second port is scheduled ([0065](0065-names-and-crates.md)).

**Modules are not part of an operation.** A module binds to operations' parameters, so module definitions live in `apertrust-modules` ([0028](0028-modules-bound-to-operations.md)).

**darktable's modules are never loaded.** Its code is the reference a port is written from, with provenance recorded per operation, and ported files keep darktable's copyright ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)).

**Third-party plugins come later** as declarative packages: WGSL kernels and WebAssembly logic behind the same interface. The engine checks their slots ([0022](0022-pipeline-order-and-the-default-pipeline.md)) and their kernels ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md)) at load.

## Options considered

- Kernels in one shared crate with thin operation crates: one operation split over two crates, away from one-unit third-party packages.
- A CPU bridge loading darktable's modules: links darktable, against [0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md), and leaves the GPU.
- Native shared-library plugins: an unstable ABI and unsafe loading.

## Consequences

- A port is one crate plus its spec in `docs/ports/<module>.md` (the `port-module` and `new-plugin` skills).
- A new operation goes into the workspace's members and `OPERATIONS`.
