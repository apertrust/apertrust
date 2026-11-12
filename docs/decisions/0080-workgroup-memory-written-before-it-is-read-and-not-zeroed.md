# 0080. Workgroup memory written before it is read, and not zeroed

## Context

WebGPU zeroes a kernel's workgroup memory before it runs, so no kernel reads what another left there, and wgpu does it by default by adding code at the kernel's start (`zero_initialize_workgroup_memory`). On Direct3D 12 that code goes through naga's HLSL and FXC ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md)), and FXC takes seconds over it beside the barriers of a reduction over shared arrays. Highlight reconstruction's `opposed_sums` and `opposed_chroma`, each a tree over 256 `float4` and 256 `uint4` entries, took 16 to 18 s and 8 to 9 s to compile, on any GPU on Direct3D 12, every time a device compiled them; on Vulkan on the RTX 5060 about 40 ms. On WARP the zeroing also ran at every dispatch: `opposed_sums` took 6.6 s on a 61 MP raw. On Metal naga zeroes from one invocation per workgroup, and a layout of haze removal's guided filter took 140 and 155 ms at 10 MP on the M4 with it, 13 and 34 ms without ([hazeremoval.md](../ports/hazeremoval.md)). Every engine test opens its own device, so on Windows' CI runner each one pays it, and the suite runs past six hours.

## Decision

**Every kernel writes each element of its workgroup memory before any invocation reads it, and the engine leaves wgpu's zeroing out** (`zero_initialize_workgroup_memory: false` in `ImageKernel::with_entry_point`, where the engine creates every compute pipeline), on every platform and backend.

- A reduction writes its invocation's own element before the first barrier; an atomic counter array is cleared with `store(0)` across the workgroup, then a barrier, before any `add`; a tile loaded into workgroup memory covers every element its reads use.
- The kernels with workgroup memory now, each checked: highlight reconstruction's sums and chroma, the scopes, filmic rgb's clipped counts, haze removal's histogram, prefix sums, ambient light and guided filter tiles, and the tone equalizer's minimum and maximum.
- The rule holds on every platform, so a kernel that breaks it reads leftover values on Linux and macOS as on Windows, and their tests can catch it, not only Windows users.

Measured on a Ryzen 7 8845HS with an RTX 5060, the 61 MP Sony ILCE-7RM5 sample's viewport and the engine's tests:

| | Zeroed | Not zeroed |
| --- | --- | --- |
| Compiling the viewport's 24 kernels, Direct3D 12 on the RTX 5060 | 25.0 s | 0.9 s |
| The same on WARP | 25.1 s | 0.87 s |
| `opposed_sums` on WARP | 6.6 s | 0.26 s |
| The session test `a_reset_writes_and_a_change_that_changes_nothing_does_not` on WARP | 78 s | 1.8 s |
| The engine's tests on WARP, release, one at a time | Cancelled after 6 h on CI's windows-2025 runner | 14 min on the Ryzen 7 8845HS, the build included; the whole workspace's in 50 min on CI's windows-2025 runner |

## Options considered

- **Keeping the zeroing:** Windows' CI past six hours, 25 s of compiling on every Direct3D 12 machine as a photo opens, and 6 s more for each render on WARP.
- **A flag each kernel sets, the zeroing on by default:** the same speed for the kernels that set it, but a second state for every pipeline to carry and test, where every kernel with workgroup memory is checked now.
- **Leaving it out on Direct3D 12 alone:** the same speed, but a kernel that breaks the rule would misbehave only on Windows, hidden on the platforms tested most.
- **DXC instead of FXC:** a C++ library Apertrust would ship ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md), decision 5), and the zeroing would still run on WARP.
- **Restructuring the two kernels:** the zeroing would stay for every other kernel with workgroup memory, and its cost on WARP with it.

## Consequences

- No check enforces the rule: review and the tests on every platform do. A port or kernel that adds workgroup memory states in its review how each element is written first (`slang-kernels`).
- Windows' tests fit CI again ([0070](0070-ci-in-three-layers.md)); they are switched on once the Windows-only failures their first run found pass (planned).
- Third-party plugins' kernels (planned) are reviewed by no one here: whether the engine zeroes their workgroup memory is decided with third-party loading ([plugins.md](../plugins.md#loading)).
