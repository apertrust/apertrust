# 0005. Rust, and no C or C++ unless no Rust option exists

## Context

Which language is Apertrust written in, and may it compile in or ship libraries in other languages? darktable is C, and many imaging libraries are C or C++. Each one compiled in brings a second toolchain, system packages to build against and gaps on some platforms. Yet a Rust program reaches the GPU, the window system and the desktop's services through libraries the platform provides.

## Decision

**Apertrust is written in Rust**, the engine, the app, the CLI and the tools alike: the compiler checks code written by people and agents, one toolchain builds every target, and the ecosystem covers the GPU, windows and file formats.

**No C or C++ library is compiled into Apertrust or shipped with it unless no Rust option exists.** In practice:

- Raw decoding is rawler ([0023](0023-rawler-its-camera-data-fixed-upstream.md)).
- Colour management is moxcms; lcms2 is only the reference its validation compares against, never compiled in or shipped ([0026](0026-colour-profiles-and-colour-management.md)).
- XMP is Apertrust's own layer on quick-xml ([0050](0050-our-own-xmp-layer.md)).
- Lens correction ports lensfun's lookup and interpolation to Rust and reads its database directly ([0025](0025-lens-correction.md)).
- Slang is a build-time tool; its WGSL output is committed, so no Slang library is compiled in or shipped ([0009](0009-slang-for-every-kernel-compiled-to-wgsl.md)).

**ONNX Runtime is the one exception so far:** a prebuilt library loaded at run time through its C API, never compiled in ([0060](0060-ai-inference-with-onnx-runtime.md)).

**Libraries the platform provides and Apertrust never ships, loaded at run time, are outside the rule:** the Vulkan loader and drivers (wgpu, through ash and libloading), libwayland, libxkbcommon and the X11 libraries (winit), libdbus (rfd's `xdg-portal` backend), and the operating system's own frameworks and system DLLs that crates link dynamically (Metal and AppKit on the Mac, Direct3D 12 and FXC's `d3dcompiler_47.dll` on Windows). No C is compiled on any desktop target.

**On Linux, a crate that loads a platform library at run time is preferred** to one that links it when built, so a desktop without the library loses that feature instead of failing to start, and the build needs no development package (rfd's `xdg-portal` backend, not its GTK one). The Linux packages ship none of these libraries: the Flatpak takes them from its runtime, the AppImage from the system.

**Anything bundled, downloaded by the app or compiled in needs its own exception**, the maintainer's to make. DXC's `dxcompiler.dll` stays unshipped ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md)); exiv2 and SQLite would each need an exception. jpeg-encoder, pure Rust under the IJG licence, is allowed and acknowledged in `CREDITS.md`.

## Options considered

- C++, for verbatim reuse of darktable's code: two languages' worth of build and safety problems for code that is ported anyway.
- C and C++ libraries compiled in where convenient (RawSpeed, lensfun, lcms2, exiv2): RawSpeed needs CMake, pugixml and OpenMP and does not build with MSVC; lensfun needs GLib.
- Every platform library needing an exception: wgpu and winit already break that, with no Rust option for a GPU loader or a compositor's client library.
- Only libraries opened with `dlopen` counting as the platform's: leaves the Mac's frameworks and Windows' system DLLs in doubt.

## Consequences

- `cargo deny check` and review hold dependencies to Rust and to GPLv3-compatible licences ([docs/dependencies.md](../dependencies.md)).
- A crate that compiles C, or a library Apertrust would bundle or download, needs a recorded exception; a crate reaching a platform library at run time does not.
- Exceptions not yet asked for: libgphoto2, for camera import and tethering, decided when camera import is planned in detail, leaning to allow it loaded at run time as darktable does, a project's watched Capture folder serving meanwhile; `rusqlite` only if spike L2 rules out both Rust options for the library's cache ([0055](0055-the-librarys-cache.md)). No embedded scripting language for now ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).
- The map's online tiles take TLS from the platform (`native-tls`: Security framework, SChannel, the system's OpenSSL), which the rule above covers, so they need no exception.
- Open: whether ONNX Runtime's exception covers the acceleration packs the app would download (vendor execution providers and the CUDA or ROCm libraries they need); a vendor runtime the user installs is the platform's.
