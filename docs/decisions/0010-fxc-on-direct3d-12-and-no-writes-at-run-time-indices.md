# 0010. FXC on Direct3D 12, and no writes at run-time indices

## Context

On Windows, wgpu compiles shaders for Direct3D 12 with FXC or DXC. wgpu's default, `Dx12Compiler::Auto`, loads DXC from any `dxcompiler.dll` on the DLL search path and falls back to FXC only without one, so users could compile with a compiler CI never runs. FXC cannot write a function-local vector component at an index known only at run time ("array reference cannot be used as an l-value; not natively addressable"): it tries to unroll the loop around the write, and naga's `force_loop_bounding` stops the unrolling. Demosaic, highlight reconstruction and the viewport's demosaic failed this way.

## Decision

**FXC, pinned.** The engine's `gpu::instance_descriptor`, which the app's window uses too, sets FXC, so every Windows machine on Direct3D 12 compiles as CI does and Apertrust never loads a C++ library it happens to find. `WGPU_DX12_COMPILER` still chooses another, for experiments. FXC ships with every Windows install and works on WARP.

**The kernel rule:** no kernel writes a local vector component or array element at a run-time index.

- Private variables count as local.
- A local array's element, and a vector component in workgroup memory, are refused too until tested on Windows.
- A workgroup array's element and anything in a storage buffer, which naga writes at a computed offset, are allowed.
- `add_at` and `set_at` in `crates/plugin/slang/components.slang` write component c by comparing c with every component's index at once and selecting, so the others keep their values exactly, infinities included.

**Enforcement:** only CI's Windows tests run FXC, every night and on pull requests that change a Windows path ([0070](0070-ci-in-three-layers.md)), so the engine's naga test (`no_kernel_writes_a_local_at_a_run_time_index`) walks naga's IR of every kernel, the engine's and every operation's, on every machine. When third-party plugins arrive, the engine runs the same check on their kernels at load.

## Options considered

- Keeping `Auto`: results could differ by machine, with a compiler CI never runs.
- DXC preferred, kernels kept FXC-safe: `dxcompiler.dll` is a C++ library to ship, an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md), and the rule would still be needed where the DLL is missing.
- DXC only, dropping the rule: fails wherever the DLL is missing or older than wgpu's minimum.
- `StaticDxc`: compiles C++ in, and is not available on Windows on ARM.

## Consequences

- Demosaic, highlights and the viewport's demosaic use `components.slang`; their output is bit-identical to before and as fast.
- The `slang-kernels` skill and the reviewer's checklist carry the rule.
- Open, each refused until tested on Windows: a write at a run-time index to a local array's element, and to a vector component in workgroup memory; the check at load, built with the plugin loader.
