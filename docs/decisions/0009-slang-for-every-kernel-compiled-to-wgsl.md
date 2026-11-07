# 0009. Slang for every kernel, compiled to WGSL

## Context

Which language are the GPU kernels written in? wgpu takes WGSL and translates it to every backend, so a kernel language is chosen for what it gives the authors, not for portability or speed: a kernel spike found Slang's WGSL no slower than hand-written WGSL. The differentiable pipeline ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md)) needs a derivative of every kernel that stays in step with its forward code.

## Decision

**Every compute kernel is written in Slang**, for its automatic differentiation, modules and generics, and compiled to WGSL.

- Gradients stay in sync with the forward code only if both come from one source. Slang generates each differentiable kernel's `<name>_backward` and `<name>_forward_diff` entry points from it.
- **The generated WGSL is committed**, with its Slang source's licence header, and validated with naga by the engine's tests on every machine. Slang is a build-time tool: `tools/fetch-slang.sh` fetches a pinned, checksum-verified `slangc` into `external/`, and `tools/build-kernels.sh` regenerates the WGSL (`--check` lists stale or orphaned files).
- An edit to generated WGSL is refused; the source is the `.slang` file.
- **Each operation's kernel lives in its operation's crate** ([0020](0020-operations-as-native-plugins-one-crate-each.md)); shared colour maths in `crates/color`; the pass header and `components.slang` in `crates/plugin`; the engine's own kernels (downscale, region copy, resample, scopes) in `crates/engine/slang`.
- **Every derivative is checked against finite differences** (`apertrust-golden gradient`), and forward and reverse mode against each other, since autodiff output is generated code nobody reads. The check has found defects in Slang's derivatives; the `slang-kernels` skill lists the traps, `pow0` rather than `pow` among them.

The app's display shader, which only draws the frame and has no derivatives, is plain WGSL validated with naga.

## Options considered

- Plain WGSL, with naga_oil imports: a second, hand-maintained backward kernel per operation, or finite differences costing 2N forward passes per gradient against roughly two to three for autodiff.
- Slang compiled natively to Metal or SPIR-V through wgpu's passthrough: no speed gain, and a second path to validate.
- A plain-WGSL fallback beside Slang: two sources for one kernel.

## Consequences

- A kernel change needs `slangc`, and the regenerated WGSL is reviewed with the change.
- Hooks regenerate and validate the WGSL after an edit to a `.slang` file.
- Porting a module includes its derivative entry points and the gradient check, or a recorded, understood gap in its port spec. The raw stage has none ([0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md)).
