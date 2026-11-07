# AI and the differentiable pipeline

What runs models, how assistants and agents reach the editor, and how the pipeline's derivatives let Apertrust set the sliders towards a goal. Every client, agents included, goes through the [session protocol](session-protocol.md); the kernels' derivative entry points are [engine.md](engine.md#later-phases)'s and the `slang-kernels` skill's.

**Status.** Built: every differentiable kernel exports its derivatives, checked against finite differences. Planned: the matching slice, Phase 3's first engine work; AI masks in Phase 4; MCP, AI denoise, AI erase and reference matching in Phase 6 (decisions 11, 58 and 60).

## AI inference

ONNX Runtime stays, with its native WebGPU execution provider as the default; vendor providers become optional downloads (decision 60). Agents connect through MCP.

**The WebGPU execution provider** runs on Dawn, which dispatches to D3D12, Vulkan or Metal, and needs no vendor SDK on the user's machine ([ORT docs](https://onnxruntime.ai/docs/execution-providers/WebGPU-ExecutionProvider.html)). On Linux it needs only the Vulkan loader, which removes the CUDA, cuDNN and ROCm installation problem. It ships as a plugin provider loaded at run time, the `ort` crate exposes it, and the same native engine serves remote web clients from a home server.

**The AI spike** runs the object mask, denoise and inpainting models on the WebGPU provider on the RTX 5060, the Radeon RX 9060, the Radeon 780M and the M4. It passes when their speed is acceptable against CUDA, MIGraphX and Core ML, with no hidden CPU fallback and the copy time measured against the inference time; otherwise a model ships only in an acceleration pack.

| Provider | Tier | Platforms |
| --- | --- | --- |
| WebGPU EP | Bundled, default | Windows, macOS, Linux |
| Core ML | Bundled | macOS |
| CPU EP | Bundled, last fallback | All |
| CUDA + TensorRT | Optional pack | Windows, Linux |
| ROCm/MIGraphX | Optional pack | Linux |
| OpenVINO | Optional pack | Windows, Linux |

**Trade-offs accepted:** slower than vendor providers on heavy models such as diffusion inpainting; unsupported operators fall back to the CPU, so every shipped model is profiled; no zero-copy, because Dawn and wgpu are separate devices. Burn on wgpu stays an option for small models inside the pipeline later.

**Where the libraries come from.** Microsoft publishes both as NuGet packages, which are zip files, so packaging takes their native files without NuGet's tools:

- [Microsoft.ML.OnnxRuntime](https://www.nuget.org/packages/Microsoft.ML.OnnxRuntime): the core `onnxruntime` library for Windows x64 and ARM64, Linux x64 and ARM64 and macOS on Apple silicon (and Android and iOS); MIT.
- [Microsoft.ML.OnnxRuntime.EP.WebGpu](https://www.nuget.org/packages/Microsoft.ML.OnnxRuntime.EP.WebGpu): the WebGPU provider as a plugin library for the same desktop platforms, with DirectX's shader compiler, `dxcompiler.dll`, on Windows; MIT, its third-party notices in the package.
- Neither has a build for Intel Macs, which get ONNX Runtime from Microsoft's release archives or run without AI features; decided with the AI spike.
- The plugin is young (0.x). Before Phase 6 is built: that the `ort` crate, loading ONNX Runtime at run time, registers a plugin provider library (or the C API is called directly), and the notices go into `CREDITS.md`.

**Acceleration packs** download on demand rather than shipping in every package: bundling CUDA and ROCm makes Flatpak and AppImage huge and complicates redistribution licences. The existing darktable work on execution providers carries over directly.

**Rejected as the default:**

- **Burn on wgpu:** zero-copy with the pipeline, but less complete ONNX import, slower kernels, and large models may not convert.
- **Candle:** weak on AMD and Vulkan.
- **tract:** CPU only.
- **ncnn:** C++, plus a model conversion step.

### Zero-copy between ONNX Runtime and the pipeline: not now

Each piece of GPU memory belongs to one GPU device. The pipeline runs on wgpu and ONNX Runtime's WebGPU provider on Dawn, so there are two devices, even on the same physical GPU. When a model needs a pipeline image, the data takes a detour: copied from wgpu's memory to system RAM, uploaded into Dawn's memory, the model run, the result downloaded and uploaded back into wgpu. That is four transfers instead of none, plus waits between them. On discrete GPUs each crosses PCIe; on unified-memory machines they are copies within the same RAM, cheaper but not free.

The cost depends on image size and how often the model runs:

- An AI mask at preview resolution, computed once: a few megabytes, a few milliseconds. Negligible.
- Full-resolution AI denoise at export: a few hundred megabytes per transfer for a 60 MP image, roughly tens of milliseconds each over PCIe 4.0. Small next to inference taking seconds.
- AI re-evaluated interactively while dragging sliders: copies can become a real share of every update.

Zero-copy means both devices use the same GPU memory allocation. On Vulkan, external-memory extensions let one device export a buffer as a handle and the other import it, with external semaphores to coordinate access; Metal and DX12 have equivalents. It would mean reaching below wgpu's portable API for native handles, importing them on Dawn's side for ONNX Runtime's inputs and outputs, exact synchronisation, and separate code and testing for Vulkan, Metal and DX12 across NVIDIA, AMD, Intel and Apple drivers. Moving the whole pipeline to Dawn would reverse the wgpu decision.

**The door stays open:**

- Model inputs and outputs are GPU storage buffers in a fixed layout, and the transfer step sits behind an interface. Today it copies; a zero-copy version can replace it later without touching anything else.
- The AI spike measures copy time against inference time for each model.
- Zero-copy is revisited only if an interactive AI feature spends more than roughly 10 to 20% of each update on copies.

## Models

**Models are pipeline nodes** producing masks or pixels. They run on the user's computer, and the editor is complete without them (decision 1, principle 8). AI erase removes small distractions by filling from their surroundings; adding or replacing content is not built.

## Agents

**MCP** through `rmcp` is a thin adapter over the session protocol (decision 58). Its tools cover listing images, reading modules and their controls, setting module values, histograms and rendering previews; MCP tools can return images, so agents can see their edits.

**Assistants are a core capability, not the headline** (decision 1, principle 8). Scripts and AI assistants work through the same modules, layers, history and undo as a person, and nothing they can do is hidden from the editor. What they change appears in the history as ordinary module values with a short note on what they did, and one undo reverts it. They check their results by measurement, such as histograms or the distance to a target, before showing them, and they connect only when the user allows it.

## The differentiable pipeline

The pipeline is differentiable-ready from the start: every differentiable kernel exports derivatives (decision 11). That is what lets Apertrust set the sliders towards a goal, the third part of its promise ([principles.md](principles.md), principle 7; decision 1). A survey of other editors, in October 2026, found none that fits editable settings through gradients of its own processing: vkdt and AutoShade fit with finite differences or classical solvers, and commercial style learning runs in the cloud on thousands of past edits.

**What it is.** Besides computing the output image from the slider values, a differentiable pipeline can compute how the output would change if each slider moved a little. Those rates of change are gradients. With them, an optimiser can adjust every slider in the edit at once towards a target, converging in a few dozen small renders instead of trial and error. It is the mechanism used to train neural networks; here the "network" is the editing pipeline and the "weights" are ordinary slider values.

**Why it matters.** The result is editable module settings, not a black-box filter. An AI style transfer produces pixels; a differentiable pipeline produces "exposure +0.4, contrast +12, these curve points, this colour grading", which the user can tweak like any other edit.

**What it enables** (principle 7):

- Match a reference look, producing real module settings.
- Consistent batch edits across a series shot under changing light.
- Better Auto buttons that optimise exposure, white balance and tone towards defined targets.
- Matching two cameras, so one body renders like another.
- Agent-driven edits: an agent turns "warmer, with more open shadows" into a target, and the optimiser finds precise slider values.
- Learning from examples, on the photographer's own computer: a look from a few of their edits, a preset from before-and-after pairs, or a 3D LUT turned into editable module settings.

**What the user keeps** (principles 7 and 8). The optimiser moves only the sliders the user has not set; those stay fixed until the user releases them. Matching has a strength control. The result is ordinary module values in the history, with a short note on what was done, and one undo reverts it.

**The matching slice** (decisions 1 and 11) is Phase 3's first engine work: a command-line test that matches a reference photo. It fits the editable settings of operations after demosaic through their kernels' derivative entry points, at preview resolution, and reports how close the result comes on real photo pairs; its pass threshold is set with the test ([principles.md](principles.md), How we will know it works). It proves the one promise no other editor makes, and it is cheap: every differentiable kernel already exports derivatives that pass the gradient check. The raw stage has none (decision 21), and the slice does not need them. It still needs derivatives with respect to each kernel's input pixel, so that gradients chain through the pipeline, which no kernel exports yet, and a pipe that runs the backward passes ([engine.md](engine.md#later-phases)).

**What darktable has instead.** darktable's pipeline is plain forward computation in C and OpenCL. It has per-module fitting tools: auto pickers in exposure, sigmoid, filmic and the tone equalizer that use image statistics; colour checker profiling in color calibration, which fits a matrix to a chart; and darktable-chart, which builds a style from a raw and JPEG pair. Each solves one narrow problem in one module; none can optimise all sliders across the whole edit. Commercial editors' Auto features, as far as we know, use trained models that predict slider values rather than optimising through the pipeline.

**How it works.** Slang's automatic differentiation generates gradient kernels from the normal kernels. Computing gradients costs roughly two to three times a normal render plus memory for intermediates, but optimisation runs at preview resolution (around 512 px), so it stays fast. Interactive editing only ever runs the normal forward pipeline.

**Rules:**

1. Write kernels so Slang can differentiate them.
2. Avoid unnecessary hard clamps, thresholds and branches in operations; where they are needed (clipping, hard-edged masks, median filters), plan smooth approximations for the optimisation path.
3. Keep operation parameters as continuous values.
4. CI checks that gradient kernels compile for the core operations.
5. Every operation's gradients match finite differences (`apertrust-golden gradient`, within 1% relative error) on its golden case and on the synthetic sweep, and its forward- and reverse-mode derivatives agree per pixel, or the gap is understood and recorded in its port spec. Autodiff output is generated code nobody reads, so this check is what makes it trustworthy: it found two defects in Slang's derivatives in the first operation checked.

Retrofitting differentiability later would mean reworking kernels written without it in mind; these rules cost little now.
