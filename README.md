<!-- markdownlint-disable MD033: HTML centres the logo and swaps it for dark mode on GitHub -->
<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="crates/app/assets/icons/svg/apertrust-logo-dark.svg">
    <img src="crates/app/assets/icons/svg/apertrust-logo.svg" alt="Apertrust" width="440">
  </picture>
</h1>
<!-- markdownlint-enable MD033 -->

> ⚠️ **A personal experiment, not ready for everyday use.** I am building Apertrust to see how useful and efficient AI coding agents can be at rewriting an application as complex as an open-source raw editor. It is not ready for stable use, and it may never be. Please use it with caution, and keep copies of your photos and edits.

**A free, non-destructive, cross-platform raw photo editor.**

Apertrust is a raw photo editor for people who care how their pictures look and want predictable results on their own computer. It is free software, in early development.

It builds on darktable's colour science. The first phases migrate darktable's processing and modules to the GPU, and each step is compared with darktable's own render until it matches. After the migration, planned for the end of Phase 3, Apertrust's own renders become its reference and its processing can change on its own terms. No darktable code is loaded or linked.

## What it aims for

These are the project's goals. The first is in place today; the others are planned for later phases.

- **Measured image quality.** Processing steps are checked by automated tests on every change, and the same edit should give the same picture on any supported computer.
- **Predictable controls.** Each slider has a written description of what it changes and what it leaves alone, with tests that check it. A module that is off leaves the photo as it was.
- **Help towards a result.** Later phases add ways to describe or show the result you want, such as a reference photo or a consistent look across a series, and let Apertrust suggest slider values that you can then adjust.
- **Local and private.** Any learning from your own edits stays on your computer.
- **Scripting and assistants.** Scripts and AI assistants work through the same sliders and history as you do.

Everything runs locally: no account, no cloud, no subscription. Edits are saved in a readable file beside the photo, and the original is never touched.

## Principles

- **Trust:**
  - quality is never traded for simplicity;
  - what you see is true;
  - your work is safe, and it is yours.
- **Ease:**
  - start from a good picture;
  - every control keeps to what it says;
  - one module for each tool, its depth inside it.
- **Help:**
  - show or tell the goal, and the editor sets the sliders;
  - help is visible, reversible and yours to accept.

The full principles, who Apertrust is for and what it will not do are in [docs/principles.md](docs/principles.md).

## Status

Early development, in Phase 2: the desktop app. [docs/roadmap.md](docs/roadmap.md) has the details, phase by phase.

- **The engine** renders raw files through the eight processing steps every photo needs, and matches darktable's render at a mean ΔE2000 of about 0.001 on 27 of the 29 comparable raw.pixls.us samples. Two lossy Sony formats wait for a decoder fix.
- **The desktop app** opens and edits one photo with sixteen migrated modules, saves the edit after each change, shows the scopes and opens a folder. On the Mac it exports through a native Export dialog.
- **The command line** edits, exports JPEG and PNG at darktable's sizes with file names from darktable's variables, and lists a folder.
- **Platforms:** macOS (Metal), Windows (Direct3D 12) and Linux (Vulkan) pass CI, with software GPUs where there is no hardware one.

## Downloads

Releases and the nightly builds of Apertrust Dev are on the [Releases page](https://github.com/apertrust/apertrust/releases). Nightlies are untested development builds. Neither is signed with a developer's certificate yet, so each system warns the first time you open it:

- **macOS** refuses the disk image, and the app in it, as from an unidentified developer, or says it "is damaged and can't be opened". Before opening the disk image, remove the download's quarantine from it in Terminal; the app copied out of it then opens too:

  ```sh
  xattr -d com.apple.quarantine ~/Downloads/Apertrust*.dmg
  ```

  Then open the disk image and drag the app into Applications. If the app still won't open, or was copied from an image before its quarantine was removed, remove the app's too:

  ```sh
  xattr -dr com.apple.quarantine /Applications/Apertrust*.app
  ```

  Or, without Terminal, where macOS names an unidentified developer: open the disk image once, then choose Open Anyway in System Settings › Privacy & Security and confirm, and do the same for the app the first time it opens. On macOS 14 and earlier, Control-click it and choose Open instead.
- **Windows**: SmartScreen says "Windows protected your PC": choose More info, then Run anyway.
- **Linux**: make the AppImage executable (`chmod +x`) and run it.

[docs/packaging.md](docs/packaging.md#unsigned-packages) says why, and what each package holds.

## How it works

- **Engine:** a GPU pipeline on wgpu (Vulkan, Metal, Direct3D 12) that keeps every intermediate image on the GPU. Without a hardware GPU it runs on a software one, more slowly.
- **Processing:** darktable's algorithms, migrated one by one to kernels in Slang. During the migration each is checked against darktable's own output by ΔE2000 before it is accepted; after it, against Apertrust's own renders.
- **Kernels** are compiled to WGSL. Slang's automatic differentiation gives every editing step its gradients, which is what lets Apertrust work out slider values for a goal.
- **Raw decoding:** rawler, in Rust.
- **Interface:** egui.

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

## Documentation

- [docs/principles.md](docs/principles.md): what Apertrust is and how it should feel to use.
- [docs/roadmap.md](docs/roadmap.md): what is built and what comes next, phase by phase.
- [docs/architecture.md](docs/architecture.md): the design in overview, linking each area's document.
- [docs/decisions/](docs/decisions/README.md): every decision behind it.
- [docs/engine.md](docs/engine.md): the processing pipeline.
- [docs/modules/README.md](docs/modules/README.md): the editing modules, one per darktable module, which are built, and the rules they follow.
- [CONTRIBUTING.md](CONTRIBUTING.md): how to contribute, and the checks a change must pass.
- [AI_POLICY.md](AI_POLICY.md): how AI-assisted contributions are treated.
- [SECURITY.md](SECURITY.md): how to report a vulnerability privately.
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md): the Contributor Covenant, which everyone taking part follows.
- [GOVERNANCE.md](GOVERNANCE.md): who decides, and how roles may grow.
- [docs/dependencies.md](docs/dependencies.md): how a dependency is chosen and tracked.

## Licence

GPL-3.0-or-later; see [LICENSE](LICENSE). Apertrust ports code from darktable, which is also GPL. The bundled IBM Plex Sans fonts are under the SIL Open Font License 1.1.
