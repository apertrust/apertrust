# 0103. Golden references from darktable's official build

## Context

Every golden reference, the per-operation cases, the raw stage's checks and the whole-pipeline test, is rendered by darktable-cli at the reference release ([0003](0003-the-reference-is-darktables-latest-stable-release.md), [0061](0061-golden-image-tests-against-darktable-cli.md), [0062](0062-the-whole-pipeline-comparison.md)). The harness checks darktable-cli's version, not how it was built. darktable builds its release configuration with `-O3 -ffast-math`, and, unless `BINARY_PACKAGE_BUILD` is on, with `-march=native` (`cmake/march-mtune.cmake`), so a build compiled for the local CPU vectorises and fuses its CPU loops in its own way. Which darktable-cli builds may render the references?

Measured on a Linux machine, an AMD Ryzen 7 8845HS (Zen 4, AVX-512) with a Radeon 780M:

- **A local 5.6.2 build with `-march=native`** (`/opt/darktable`): the four filtered tone equalizer cases (`toneequal-eigf`, `-average`, `-power`, `-eigf-iterations`) fail their tolerances: all four the maximum of 0.050, three of them the mean of 0.010 too (means of 0.0099 to 0.0197, maxima up to 1.06) and hundreds of thousands of pixels over. In that build the plugin's EIGF and luminance-mask loops (`eigf_variance_analysis`, `eigf_blending`, `luminance_mask`) are AVX-512 code with fused multiply-adds.
- **The difference is darktable's mask,** up to 0.06 EV in flat areas and on saturated labels, not Apertrust's code: the first public commit fails the same way, and the CPU restatement of the filters in Rust (`crates/op/toneequal/src/filters.rs`) gives the GPU kernel's numbers.
- **darktable's official `Darktable-5.6.2-x86_64.AppImage`,** from the release-5.6.2 GitHub release: all seven tone equalizer cases pass, the four filtered ones among them, at means of 0.0002 to 0.0013, maxima of at most 0.0037 and no pixel over. Its EIGF and luminance-mask loops are baseline x86-64 code with no fused multiply-adds; the functions darktable marks `__DT_CLONE_TARGETS__` carry run-time clones, from `default` to `avx512f`, chosen by the CPU.
- **The Apple M4's references,** against which the port specs' results are recorded, came from darktable's official macOS build.

## Decision

**Golden references are rendered only by darktable's official, generic build of the reference release, never by one compiled for the local CPU.** The rule covers every reference the harness renders: `render`, `run`, the raw stage's commands, `pipeline` and `export`.

**Which builds count:**

- darktable's official release packages for the reference release, from its GitHub release: the Linux AppImage (`Darktable-X.Y.Z-x86_64.AppImage`, `-aarch64` on Arm), the macOS disk image (`darktable-X.Y.Z-arm64.dmg`) and the Windows installer (`darktable-X.Y.Z-win64.exe`, `-woa64` on Arm);
- a distribution's package or a local build only if it is built generically, with `BINARY_PACKAGE_BUILD=ON` (`-mtune=generic` instead of `-march=native`), at the reference release's tag, and only when the official package cannot run on the machine.

**How the harness reaches it:**

- **The harness never looks in `/opt/darktable`,** where darktable's `build.sh` installs a build compiled for the CPU by default: after `DARKTABLE_CLI` and `PATH` its default locations are the official packages' install folders alone, macOS's and Windows', and on Linux `DARKTABLE_CLI` or `PATH` names the official build.
- **Linux:** unpack the AppImage with `--appimage-extract` into a folder of its own (it writes `squashfs-root/`), and point `DARKTABLE_CLI` at a small wrapper script that runs the unpacked `usr/bin/darktable-cli` through the AppImage's hooks: `APPDIR` set to `squashfs-root`, each script in `squashfs-root/apprun-hooks/` sourced, then `exec` of `squashfs-root/usr/bin/darktable-cli "$@"`. The AppImage's own `AppRun` starts the GUI, so it cannot stand in for darktable-cli.
- **macOS:** install the disk image's `darktable.app` in `/Applications`; the harness finds `/Applications/darktable.app/Contents/MacOS/darktable-cli` by default.
- **Windows:** install with the official installer; the harness finds `C:\Program Files\darktable\bin\darktable-cli.exe` by default.

**References rendered by another build are discarded,** not kept beside the official ones. The harness keys its cache by the reference version, the render options, the sidecars and the raw, not by the build, so switching builds means deleting `target/golden` before rendering again.

**Results recorded in a port spec** name the darktable build beside the GPU where it is not the official package.

## Options considered

- **Any build of the reference release, with wider tolerances where a local build differs:** rejected. It would loosen every case to cover darktable's own build variation, hiding real errors of the size the tolerances exist to catch, and the tolerance would depend on the machine that rendered the reference, against principle 1's same image on every computer.
- **Any build, with references re-rendered per machine and failures explained case by case:** rejected. A failure would first have to be told apart from darktable's compiler, as the tone equalizer's took, on every machine and every port.
- **References rendered once, on one machine, and shared:** not chosen now. It removes the build's variation altogether, but full-resolution references are too large to commit and need a store; it belongs with where Apertrust's own baselines are kept after the migration ([testing.md](../testing.md#open-questions)).

## Consequences

- AGENTS.md's golden-image harness paragraph and [testing.md](../testing.md#the-golden-image-harness-during-the-migration) say the references need darktable's official build, and how to install it on each platform.
- `DarktableCli::locate` (`crates/golden/src/darktable.rs`) drops `/opt/darktable/bin/darktable-cli` from its default locations, so a machine with a local build in `/opt/darktable` no longer renders references with it unasked.
- The tone equalizer's filtered cases pass on the Radeon 780M against the official AppImage ([docs/ports/toneequal.md](../ports/toneequal.md), Results). Apertrust's tone equalizer and its tolerances are unchanged.
- Even the official Linux build picks its cloned functions by the CPU at run time, so its references on an x86 CPU without AVX-512 are not yet measured against those on one with it (testing.md, Open questions).
- The `golden-tests` skill, in the agent kit, says the same in its requirements.
