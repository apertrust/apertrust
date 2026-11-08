# 0007. A software GPU instead of a CPU path

## Context

Every operation runs on the GPU ([0008](0008-a-gpu-resident-fp32-pipeline-on-wgpu.md)). What happens on a machine without a usable GPU? darktable keeps a CPU version of every module beside its OpenCL one, and the two drift apart in places. A software GPU, a driver that implements the graphics API on the CPU, runs the same kernels but binds far less memory at once: lavapipe binds 128 MiB, WARP 2047 MiB, where the hardware GPUs measured bind 2047 MiB (an RTX 5060, a Radeon 780M) to 4095 MiB (the Apple M4).

## Decision

**There is no CPU code path.** Without a hardware GPU, wgpu runs the same WGSL on a software GPU: lavapipe, Mesa's Vulkan driver, on Linux, and WARP, the software Direct3D built into Windows. Every Mac with a current macOS has Metal.

- The engine takes a hardware adapter when there is one and the software one otherwise, in the app and the CLI alike, so kernels, buffers and plans stay the same and the image matches within fp32 rounding.
- CI runs every GPU test on lavapipe and WARP, lavapipe on every pull request and WARP every night ([0070](0070-ci-in-three-layers.md)), so the golden tests cover the fallback.
- Interactive editing needs Vulkan, Metal or DX12 on real hardware. The app tells the user when its adapter reports `DeviceType::Cpu`, and points to the CLI for batch work if editing is too slow.

**Row strips and tiles carry large images onto a small binding** ([0013](0013-tiled-exports-equal-to-untiled-ones.md), [0014](0014-row-strips-for-images-over-one-binding.md)). **What still does not fit is refused, naming the pass and the sizes.** These refusals are accepted limits of software GPUs. On lavapipe's 128 MiB binding:

| Refused | Above about | Why |
| --- | --- | --- |
| A raw whose highlight masks exceed one binding | 300 MP | darktable's read past the dilated mask wraps to its first rows, so the masks stay whole |
| A viewport image | 8.4 MP of RGBA | No tile helps a viewport larger than one binding |
| Any raw, on lavapipe and WARP | 536 MP | Its float mosaic exceeds the 2 GiB buffers they make |
| Haze removal | 8 MP | Its whole-image copy, 16 bytes a pixel, which no tile makes smaller |
| The tone equalizer's grid | Smoothing typed below about 0.1 % of the image (16.7 MP at a radius of 1, 67 MP at 2, 151 MP at 3); any smoothing above 268 MP | The grid's line passes walk whole lines and cannot run in strips |
| A spatial operation whose reach leaves no tile | 8.4 MP, with a reach above about 1,400 px | Non-local means near the top of its soft range |
| Bloom, and soften | 33.5 MP | Column passes walk whole columns in darktable's order |
| Filmic rgb's highlight reconstruction | 8.4 MP (37 MP once image-shaped scratch runs in strips) | Its float4 scratch; the vertical B-spline reads 2^scales rows each way |
| The colour equalizer with its guided filter, its default | 16.8 MP | Float2 scratch bound whole for column passes; the 100% view too |
| Denoise's wavelets | 8.4 MP, until image-shaped scratch runs in strips | Three RGBA images of scratch |

A later port's refusal on a software GPU joins this list, stated in its spec with the pass and the sizes, while its port and module go ahead.

**Image-shaped scratch bound in windows** ([0015](0015-whole-image-passes.md)) is planned engine work before Phase 3's exit, lifting the refusals of local contrast, bloom, soften, the colour equalizer, filmic rgb and haze removal. **The recorded remedies** come with the ports that need them: transposed line passes (a block transpose so each column becomes a row walked whole in row strips, darktable's sums bit for bit), which lift bloom's, soften's, filmic's and the colour equalizer's refusals; non-local means' offsets in groups, each binding only the window it reaches. The other refusals stay accepted limits.

**The acceptance holds for software GPUs only.** A hardware GPU that binds 128 MiB or less, or running the engine on a phone, reopens it.

## Options considered

- A CPU version of every operation: doubles each port and its gradient checks, and drifts from the GPU path.
- Slang's CPU target: compiling its generated C++ or shipping LLVM, against [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).
- darktable's tiling with overlaps for the colour equalizer: differs from the untiled render near tile edges and breaks [0013](0013-tiled-exports-equal-to-untiled-ones.md)'s bit-identity.
- A coarser grid where the binding is small: a different image on that computer.
- A second window at the top of the image for highlight masks: a special case in one operation for composites on a software GPU.

## Consequences

- `apertrust-cli --max-binding 128` renders on any machine as lavapipe would, so the limits are tested everywhere.
- The `timing` workflow times exports and viewport changes on lavapipe and WARP.
- Open: wgpu's OpenGL backend, for GPUs without Vulkan, is untested.
