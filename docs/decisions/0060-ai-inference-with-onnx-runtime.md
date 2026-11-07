# 0060. AI inference with ONNX Runtime

## Context

Object masks, denoise and AI erase need neural network inference on the person's GPU, on Windows, macOS and Linux, without asking people to install CUDA, cuDNN or ROCm. The pipeline runs on wgpu ([0008](0008-a-gpu-resident-fp32-pipeline-on-wgpu.md)); models come as ONNX files.

## Decision

**ONNX Runtime, loaded at run time through its C API, the one exception to the Rust-only rule** ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)), through the `ort` crate. Planned.

**Its WebGPU execution provider is the default.** It runs on Dawn, which dispatches to D3D12, Vulkan or Metal, and needs no vendor SDK; on Linux only the Vulkan loader. It ships as a plugin provider loaded at run time, and the same native engine serves remote clients from a home server ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

| Provider | Tier | Platforms |
| --- | --- | --- |
| WebGPU | Bundled, default | Windows, macOS, Linux |
| Core ML | Bundled | macOS |
| CPU | Bundled, last fallback | All |
| CUDA and TensorRT | Optional pack | Windows, Linux |
| ROCm/MIGraphX | Optional pack | Linux |
| OpenVINO | Optional pack | Windows, Linux |

**Vendor providers are optional packs** downloaded on demand, so packages stay small and redistribution licences simple.

**The AI spike** runs the object mask, denoise and inpainting models on the WebGPU provider on the RTX 5060, Radeon RX 9060, Radeon 780M and Apple M4; it passes when their speed is acceptable against CUDA, MIGraphX and Core ML, with no hidden CPU fallback and the copy time measured against the inference time; otherwise that model ships only in a pack. Every shipped model is profiled for operators that fall back to the CPU.

**No zero-copy yet.** The pipeline (wgpu) and ONNX Runtime (Dawn) are two devices even on one GPU, so a model's input and output are copied through system memory: negligible for a mask at preview size, tens of milliseconds for a full-resolution denoise at export, small beside seconds of inference. The door stays open: model inputs and outputs are GPU storage buffers in a fixed layout, and the transfer sits behind an interface that a zero-copy version (external memory on Vulkan, Metal and DX12) can replace without touching anything else. Revisit only if an interactive AI feature spends more than about 10 to 20% of each update on copies.

**Models are pipeline nodes** producing masks or pixels. They run on the person's computer, and the editor is complete without them (principle 8). AI erase removes small distractions by filling from their surroundings; adding or replacing content is not built.

## Options considered

- Vendor providers bundled: huge Flatpak and AppImage packages and harder licences.
- Burn on wgpu as the default: zero-copy with the pipeline, but less complete ONNX import, slower kernels, and large models may not convert; kept as an option for small models inside the pipeline.
- Candle (weak on AMD and Vulkan), tract (CPU only), ncnn (C++ and a conversion step).
- Zero-copy interop now: native handles below wgpu's API and separate code for three backends across four vendors.
- The whole pipeline on Dawn: reverses the choice of wgpu.

## Consequences

- The AI spike sets which models ship by default.
- A model's results enter edits as ordinary module values or `.apdata` entries ([0049](0049-the-apdata-file-as-darktables-dtdata.md)).
