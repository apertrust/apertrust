<!-- markdownlint-disable MD033: HTML centres the logo and the badges, and swaps the logo for dark mode on GitHub -->
<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/app/assets/icons/svg/apertrust-logo-dark.svg">
    <img src="crates/app/assets/icons/svg/apertrust-logo.svg" alt="Apertrust" width="440">
  </picture>
</h1>
<p align="center">
  <a href="https://github.com/apertrust/apertrust/actions/workflows/ci.yml"><img src="https://github.com/apertrust/apertrust/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="https://github.com/apertrust/apertrust/actions/workflows/codeql.yml"><img src="https://github.com/apertrust/apertrust/actions/workflows/codeql.yml/badge.svg?branch=main" alt="CodeQL"></a>
  <a href="https://sonarcloud.io/summary/new_code?id=apertrust_apertrust"><img src="https://sonarcloud.io/api/project_badges/measure?project=apertrust_apertrust&amp;metric=alert_status" alt="SonarQube Cloud quality gate"></a>
  <a href="CONTRIBUTING.md"><img src="https://img.shields.io/badge/DCO-signed--off-green" alt="DCO: every commit signed off"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/apertrust/apertrust" alt="Licence: GPL-3.0"></a>
  <a href="https://github.com/apertrust/apertrust/releases/tag/nightly"><img src="https://img.shields.io/github/release-date-pre/apertrust/apertrust?label=nightly" alt="Nightly build"></a>
  <a href="#requirements"><img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-blue" alt="Platforms: macOS, Windows, Linux"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Fapertrust%2Fapertrust%2Fmain%2Frust-toolchain.toml&amp;query=%24.toolchain.channel&amp;label=rust&amp;color=orange" alt="Rust version"></a>
</p>
<!-- markdownlint-enable MD033 -->

> ⚠️ **An experiment, not ready for everyday use.** We are building Apertrust to see how useful and efficient AI coding agents can be at rewriting an application as complex as an open-source raw editor. It is not ready for stable use, and it may never be. Please use it with caution, and keep copies of your photos and edits.

**A free raw photo editor that shows its work**

Every tool says what it is for, and every edit says what it changed. The picture you see is the picture you export, the same on every computer and after every update, and its quality is measured rather than claimed. When Apertrust helps, it sets ordinary sliders and tells you why. It starts simple, with a good picture on opening and a few clear tools, and the full depth of each tool is there, one level down, when you want it.

It runs on your own computer, on macOS, Windows and Linux: no account, no cloud, no telemetry. The original file is never touched; your edit is a readable file beside it, with its whole history.

## Principles

1. **Show the work.** Every tool says what it does and when to use it, in plain words, with the technical facts underneath. Every change appears in the history, saying what it changed.
2. **Proven, not claimed.** Image quality is measured against a reference on every change, never judged by eye. An edit looks the same on every computer and after every update; a deliberate change to how photos render is a reviewed, recorded decision.
3. **Every control keeps its promise.** What each control changes, what it leaves alone and what its default does are written down and tested. A tool that is off leaves the photo untouched.
4. **Grow, never strip.** A good picture and a few clear tools first; depth inside the same tool, never removed to make things simpler. One clear way to do each thing.
5. **Help sets sliders, and you decide.** Auto, matching a photo, keeping a series consistent or an instruction in words all end as ordinary slider values in the history, with a note saying why, one undo away. Sliders you set yourself stay as you set them.
6. **Your work is yours.** Edits live beside the photo, saved as you go. Everything runs on your machine; AI runs locally, and the editor is complete without it.
7. **The project shows its work too.** Every change to the code is checked against references and tested invariants, every decision is recorded with its reasons, and AI assistance in development is disclosed, with a person accountable for every line.

When they conflict: picture quality wins over convenience, predictable over clever, and your own choice over automation.

## Built on darktable

Apertrust's processing starts from [darktable](https://www.darktable.org/)'s colour science and tools, ported to the GPU and checked against darktable's own renders until they match, with darktable's copyright and provenance kept in every ported file. Once that migration is complete, at the end of Phase 3, Apertrust's own renders become its reference and it evolves its own way. Nothing of darktable is loaded or linked.

## Status

Early development, in Phase 2: the desktop app. [docs/roadmap.md](docs/roadmap.md) has the details, phase by phase.

- **The engine** renders raw files through the eight processing steps every photo needs, and matches darktable's render at a mean ΔE2000 of about 0.001 on 27 of the 29 comparable raw.pixls.us samples. Two lossy Sony formats wait for a decoder fix.
- **The desktop app** opens and edits one photo with sixteen migrated tools, saves the edit after each change, shows the scopes and opens a folder. On the Mac it exports through a native Export dialog.
- **The command line** edits, exports JPEG and PNG at darktable's sizes with file names from darktable's variables, and lists a folder.
- **Platforms:** macOS (Metal), Windows (Direct3D 12) and Linux (Vulkan) pass CI, with software GPUs where there is no hardware one.

## Requirements

| | Systems | Graphics |
| --- | --- | --- |
| macOS | 11 or later, on Apple silicon or Intel | Metal |
| Windows | 64-bit, on x64 or Arm | Direct3D 12, or Vulkan |
| Linux | x86_64 or aarch64, with glibc 2.39 or later (Ubuntu 24.04, Fedora 40 and later) for the packages | Vulkan |

- **A GPU** does the processing, with 2 GB of its memory for photos by default (Settings › Processing). Without a hardware GPU, Apertrust runs on a software one, lavapipe on Linux or WARP on Windows, which works but is much slower.
- **Building from source** needs Rust, which rustup installs at the version `rust-toolchain.toml` names ([Building](#building)).

## Installation

Releases and the nightly builds, Apertrust Nightly, are on the [Releases page](https://github.com/apertrust/apertrust/releases). Nightlies are untested development builds. Neither is signed with a developer's certificate yet, so each system warns the first time you open it:

- **macOS** refuses the disk image as from an unidentified developer, or says it "is damaged and can't be opened".
  - Before opening the disk image, remove its quarantine, then drag Apertrust into Applications:

    ```sh
    xattr -d com.apple.quarantine ~/Downloads/Apertrust*.dmg
    ```

  - If the app still won't open, remove the app's quarantine too:

    ```sh
    xattr -dr com.apple.quarantine /Applications/Apertrust*.app
    ```

  - Without Terminal: open it once, then choose Open Anyway in System Settings › Privacy & Security (on macOS 14 and earlier, Control-click it and choose Open).
- **Windows** shows SmartScreen's "Windows protected your PC".
  - **The installer:** choose More info, then Run anyway.
  - **The portable zip:** unblock it before extracting, or each program in it warns again: right-click › Properties › Unblock.
  - **Smart App Control,** where Windows 11 has it on, blocks unsigned apps with no Run anyway.
- **Linux** has no warning to pass.
  - **The AppImage:** make it executable and run it. If it names FUSE as it fails, install `libfuse2` (`libfuse2t64` on Ubuntu 24.04). Apertrust Nightly's AppImage updates with [AppImageUpdate](https://github.com/AppImageCommunity/AppImageUpdate).

    ```sh
    chmod +x ~/Downloads/apertrust*.AppImage
    ```

  - **The tarball** installs into `~/.local`, with a menu entry and Open With for raw files:

    ```sh
    tar -xzf ~/Downloads/apertrust*.tar.gz -C ~/.local --strip-components=1
    update-desktop-database ~/.local/share/applications
    update-mime-database ~/.local/share/mime
    ```

## How it works

- **Engine:** a GPU pipeline, on Vulkan, Metal or Direct3D 12, that uploads the raw once and keeps every intermediate image on the GPU. Without a hardware GPU it runs on a software one, more slowly.
- **Processing:** darktable's algorithms, migrated one by one to GPU kernels. During the migration each is checked against darktable's own output by ΔE2000 before it is accepted; after it, against Apertrust's own renders.
- **Gradients:** every kernel is differentiable, so Apertrust can work out which slider values reach a goal, rather than guessing.

## Dependencies

Apertrust is built on Rust libraries, and adds a C or C++ library only where no Rust one exists. Every licence is compatible with GPLv3, which `cargo deny check` checks on every change. The main ones:

| Library | For |
| --- | --- |
| [wgpu](https://wgpu.rs/) | The GPU: Vulkan, Metal and Direct3D 12 |
| [Slang](https://shader-slang.org/) | Writing the kernels, compiled to WGSL; a build tool, not shipped |
| [rawler](https://github.com/dnglab/dnglab) | Decoding raws, in Rust |
| [moxcms](https://github.com/awxkee/moxcms) | Colour management (ICC profiles), validated against lcms2 |
| [egui](https://github.com/emilk/egui) | The interface |

Everything shipped, with its licence, is listed in [CREDITS.md](CREDITS.md); [docs/dependencies.md](docs/dependencies.md) says how each is chosen and why these.

## Building

You need Rust (rustup installs the version in `rust-toolchain.toml`) and a GPU, or a software Vulkan or Direct3D adapter.

```sh
cargo build --release --workspace
cargo test --workspace
cargo run --release -p apertrust-cli -- info photo.ARW                          # the raw's metadata
cargo run --release -p apertrust-cli -- edit photo.ARW --set exposure.exposure=0.5   # save an edit beside the raw
cargo run --release -p apertrust-cli -- export photo.ARW -o photo.jpg            # render to an sRGB JPEG (or .png), from the saved edit
cargo run --release -p apertrust -- photo.ARW                                    # the desktop app
```

- **Diagnostics:** add `-d perf`, or `-d all`, for timings and other diagnostics.
- **Kernels:** the generated WGSL is committed, so building needs no Slang compiler. `tools/build-kernels.sh` regenerates it after a kernel changes.
- **API docs:** `tools/build-api-docs.sh --open` builds the API docs from the code into `target/doc`.

## Contributing

Apertrust is in early development, and contributions are welcome. [CONTRIBUTING.md](CONTRIBUTING.md) has the details; in short:

- **Open an issue first** for a feature, a darktable port or any large change, so the scope is agreed before the code. A small fix needs none.
- **One small change per pull request,** linked to its issue, with the checks passing.
- **Sign off every commit** (`git commit -s`), certifying the [Developer Certificate of Origin](https://developercertificate.org/).
- **AI-assisted work is welcome** under the same rules, with you accountable for every line ([AI_POLICY.md](AI_POLICY.md)).

## Documentation

- [docs/principles.md](docs/principles.md): what Apertrust is and how it should feel to use.
- [docs/roadmap.md](docs/roadmap.md): what is built and what comes next, phase by phase.
- [docs/architecture.md](docs/architecture.md): the design in overview, linking each area's document.
- [docs/decisions/](docs/decisions/README.md): every decision behind it.
- [docs/engine.md](docs/engine.md): the processing pipeline.
- [docs/tools/README.md](docs/tools/README.md): the editing tools, one per darktable module, which are built, and the rules they follow.
- [CONTRIBUTING.md](CONTRIBUTING.md): how to contribute, and the checks a change must pass.
- [AI_POLICY.md](AI_POLICY.md): how AI-assisted contributions are treated.
- [SECURITY.md](SECURITY.md): how to report a vulnerability privately.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md): the Contributor Covenant, which everyone taking part follows.
- [GOVERNANCE.md](GOVERNANCE.md): who decides, and how roles may grow.
- [docs/dependencies.md](docs/dependencies.md): how a dependency is chosen and tracked.

## Licence

GPL-3.0-or-later; see [LICENSE](LICENSE). Apertrust ports code from darktable, which is also GPL. The bundled IBM Plex Sans and IBM Plex Mono fonts are under the SIL Open Font License 1.1. The logo's letters are set in Fraunces, also under the SIL Open Font License, and outlined, so the font itself is not shipped.
