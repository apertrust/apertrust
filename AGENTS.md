# Apertrust

A free, non-destructive, cross-platform raw photo editor ([docs/principles.md](docs/principles.md)), in Rust on the GPU with wgpu, with an editing-driven, layer-based UI. It builds on darktable's trusted colour science, migrated first and then evolved on its own (architecture decision 2). Licence GPL-3.0-or-later.

## Read first

- [docs/principles.md](docs/principles.md): what Apertrust is and how it should feel; product decisions follow it (decision 1).
- [docs/roadmap.md](docs/roadmap.md): the public roadmap by phase, built items checked, and the one place for the plan: the phases with their exit criteria and gates, and the library's steps. Update it when the plan changes or an item lands.
- [docs/decisions/](docs/decisions/README.md): every architecture decision, one file each (`NNNN-short-title.md`), listed in the index. A new decision is a new file and a line in the index (decision 66; the `decision-log` skill).
- [docs/architecture.md](docs/architecture.md): the overview: the big picture, the rules that shape everything, the crates and the engine–UI boundary, a paragraph per area linking its document, and a pointer to the plan in the roadmap. The current state by area is in [docs/decisions/](docs/decisions/README.md#the-current-state-by-area); each area's open questions end its document.
- [docs/crates.md](docs/crates.md): each crate's state, built and planned.
- [docs/engine.md](docs/engine.md): the pipeline: the GPU and its rules, software GPUs, planning, regions, pipes, caching, tiling, the module layer, data files, the Phase 1 build order (decision 12; fusion's mechanism still open).
- [docs/plugins.md](docs/plugins.md): the operation interface, one crate per operation, manifests and schemas, passes, slots, the image context, third-party plugins (planned).
- [docs/settings.md](docs/settings.md): the settings schema and store, `--conf`, the user data folders, edits in another folder (decision 43).
- [docs/library.md](docs/library.md): the library: one photo, folder mode, the Library and the Project, the code's four layers, `LibraryCommand` and `LibraryEvent`, listing and the index, sidecars, finding photos on several computers, a server, the cache and thumbnails (decisions 46 to 55 and 71).
- [docs/events.md](docs/events.md): the event API (decision 59). [docs/session-protocol.md](docs/session-protocol.md): the protocol every client reaches the engine through.
- [docs/gui.md](docs/gui.md): the GUI's architecture (decision 36): the design system, themes, Fluent, actions, accessibility, settings, UI tests; a dark default theme, British English (the source), American English, Ukrainian and Polish.
- [docs/layers.md](docs/layers.md): layers and masks as masked module instances, and the scene and display domains (decision 34, planned).
- [docs/ai.md](docs/ai.md): AI inference with ONNX Runtime, models, agents and MCP, the differentiable pipeline (decisions 11 and 60).
- [docs/testing.md](docs/testing.md): the golden-image harness, the migration's baselines and after, departures from darktable (decisions 2 and 61 to 63).
- [docs/modules/README.md](docs/modules/README.md): the editing modules, one per darktable module in the first pass (decision 2), both white balances among them (decision 27), tone mappers modules of their own; the rules every module follows, and each module's invariants, every row a test.
- [UI wireframe](https://claude.ai/artifact/V4r2xReSqn344AQstUgeiK): the current design of the whole interface. Where it differs from the modules document or `ui-conventions`, follow the wireframe.
- [CONTRIBUTING.md](CONTRIBUTING.md) (an issue first for a feature, a port or a large change; one small change per pull request), [AI_POLICY.md](AI_POLICY.md) (AI-assisted work under the same rules, the contributor accountable for every line; decision 67), [SECURITY.md](SECURITY.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), [GOVERNANCE.md](GOVERNANCE.md).
- [docs/dependencies.md](docs/dependencies.md): how a dependency is chosen (Rust first, decision 5; a GPLv3-compatible licence; on crates.io) and tracked, the main libraries and why, the licence.
- [docs/packaging.md](docs/packaging.md): the packages on each platform, built by cargo-packager from `packaging/Packager.toml` (decision 68), what they hold, how to build one, and opening an unsigned one.
- The API docs: rustdoc's, built by `tools/build-api-docs.sh` into `target/doc`. `docs/` holds only the design docs, written by hand.
- Working material stays out of this repository (decision 67): research, deliberation, session reports, product ideas, usability-test notes. What is decided is recorded here.
- `AGENTS.local.md`, git-ignored, holds this checkout's personal notes (paths, machines, GPUs, preferences) and private pointers; read it when it exists (harnesses that follow `@` imports load it here: @AGENTS.local.md).

**Current phase: Phase 1, engine and CLI** (build order in [docs/engine.md](docs/engine.md)).

- Exit criteria: every raw.pixls.us sample both decoders read as a colour mosaic (29 of the 40), rendered as `apertrust-cli export` does before rounding to 8 bits, within a mean ΔE2000 of 0.01 of darktable's 16-bit render and at most 0.01% of pixels over 2.3, with Apertrust's crop and levels on both sides (decisions 23 and 62).
- 27 pass (`apertrust-golden pipeline`); Sony's two lossy ARWs wait for rawler's dither-seed fix.
- WARP is timed. lavapipe binds at most 128 MiB at once, less than the 61 MP raw's 233 MiB mosaic, so such passes run in row strips and the rest in tiles (decision 14), byte-identical on the M4 at that limit; its timing waits for the workflow's next run.
- Phase 0's gate has passed: every golden case on the M4, an RTX 5060 and a Radeon 780M, and a fast viewport on macOS and Linux.

## Project skills

The skills, agents and commands live in the agent kit, [apertrust/agent-kit](https://github.com/apertrust/agent-kit), version 0.1.0 for this checkout. It is optional (a Claude Code plugin, or its `install.sh` into the git-ignored `.claude/` or `.agents/`); the rules are in this file, checked by CI and review either way. Follow a skill when its task comes up:

- `port-module`: a darktable port, from the spec to the golden and gradient checks.
- `golden-tests`: the harness and the raw corpus: cases, ΔE results, tolerances, when a baseline may change.
- `slang-kernels`: kernels: the interface, fp32 and uniform rules, differentiability, Slang's derivative traps.
- `desktop-architecture`: crates, the engine–UI boundary, threads, GPU resources, egui, platform code, errors, profiling, packaging.
- `decision-log`: recording a decision so the log, the summary, the sections, AGENTS.md and the skills agree.
- `new-plugin`, `tool-bindings`, `ui-conventions`: engine plugins; module bindings, invariants and transfer functions; the UI's look and behaviour.

**Agents.** `product-manager` orchestrates, run as the main session (`claude --agent product-manager`): it keeps the roadmap and the Planka board (where: `AGENTS.local.md`), plans the work, runs the other agents and brings decisions to the maintainer on the board. `architect` weighs options and records decisions, never writing code. `developer`, `porter` (one darktable module, spec to passing checks), `reviewer` (read-only), `tester` (runs checks, never fixes) and `ux-designer` (never code) take whole tasks. Run several porters or developers at once in separate worktrees.

**Small, focused tasks:**

- **Whoever delegates** gives each agent one deliverable, the smallest that meets the need: what to deliver, the files it may touch, and what is out of scope. Never an open-ended permission such as "change the CLI if it helps". Bigger work is a sequence of small tasks, each reviewed before the next. Settle the scope before launching; if it must change, stop and brief again once.
- **Whoever does the work** delivers what was asked, the simplest working version first (one case, one path), extended only when asked. The code is simple, well structured and maintainable, never a rewrite of what the task did not ask to change. Anything else worth doing is reported as a suggestion, not done.
- **Before merging**, the change's size is checked against what was asked.

**Commands.** `/port <module>` (a porter in its own worktree, then the reviewer), `/golden [case...]` (the ΔE, timing and gradient checks against the port specs' results), `/draft-commit` and `/draft-pr` (written to the git-ignored `PR.md`; opens the PR only when asked). Every skill is also a command of its own name.

**Hooks** in `.claude/settings.json`, scripts in `.claude/hooks/` (they need `jq`):

- a Rust file Claude writes or edits is formatted with rustfmt;
- an edit to a `.slang` file regenerates the WGSL and validates every kernel with naga; an edit to generated WGSL is refused with a pointer to its source;
- at the end of a turn that changed Rust, the kernels, the engine or the golden harness: `cargo fmt --check`, clippy, then the golden checks, ΔE and gradients where `APERTRUST_TEST_IMAGES` is set (put it in `.claude/settings.local.json`), otherwise `gradient --synthetic`. Failures send Claude back to fix them; changes made before the turn never trigger them.

The agent kit adds DeepWiki's public MCP server for questions about darktable, wgpu, Slang, egui and rawler (its README says how). It indexes their current branches, so it is for orientation only: a port reads darktable's source at the reference release's tag. Apertrust's own MCP server, `apertrust-cli mcp`, is planned.

## Commands

```sh
cargo build --workspace
cargo test --workspace                     # GPU tests need an adapter; software ones count
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
tools/build-api-docs.sh [--open]           # API docs into target/doc; a rustdoc warning fails, as in CI
cargo deny check                           # licences, advisories; before adding a dependency
                                           # (install: cargo install --locked cargo-deny)
tools/check-ui-strings.sh                  # fails on an English literal in interface code (CI)
tools/check-windows-paths.sh [--changed REVS]   # a platform cfg missing from .github/windows-paths.txt (CI)
tools/credits.sh [--check]                 # CREDITS.md from Cargo.lock; --check fails when stale (CI)
tools/package.sh [--finder-layout] [--dev] # this platform's packages into target/package/<target>/out;
                                           # --dev: Apertrust Dev's, as the nightly builds are
                                           # (install: cargo install --locked cargo-packager --version 0.11.8)
tools/build-icons.sh                       # the app icon's PNGs, .icns and .ico from its SVGs; commit them
tools/build-dmg-background.sh              # the disk image's background.tiff from its SVG, on a Mac; commit it (docs/packaging.md)

# The CLI; every flag in docs/crates.md, apertrust-cli
cargo run -p apertrust-cli -- gpu [--adapter NAME]   # adapters, and the one the engine opens
cargo run -p apertrust-cli -- info FILE              # decode a raw, show its metadata
cargo run --release -p apertrust-cli -- edit FILE [--set MODULE.CONTROL=VALUE] [--reset MODULE.CONTROL] [--param OP.PARAM=VALUE] [--reset-param OP.PARAM] [--clear]
cargo run --release -p apertrust-cli -- export FILE... [-o PATTERN] [--width W --height H --hq BOOL --upscale BOOL] [--set ...] [--param ...] [--skip OP] [--tile N] [--runs N]
cargo run --release -p apertrust-cli -- preview FILE [--width W --height H] [--exposure EV] [--png OUT] [--join OPERATION] [--scopes[=totals|histogram|waveform|vectorscope]]
cargo run --release -p apertrust-cli -- -d perf preview FILE --beside-export ROUNDS [--changes N] [--interval MS]
cargo run --release -p apertrust-cli -- library scan DIR                       # one folder's raws, sidecars, edits
cargo run --release -p apertrust-cli -- --max-binding 128 export FILE -o OUT.png   # as on lavapipe
cargo run -p apertrust-cli -- --conf engine.memory_budget=1024 -d settings gpu     # a setting for one run
tools/compare-cpu.sh [--raws 40d,7rv,xt5] [--paths dt-cpu,dt-opencl,software,gpu] [--runs N]
tools/compare-cpu.sh --raws 7rv --add toneequal-eigf,bilat [--size PX --hq BOOL] [--wait-quiet PERCENT] --runs 5

# The app and the design system
cargo run --release -p apertrust -- [--user-data-dir DIR] FILE
cargo run -p apertrust-ui --example gallery    # tokens, themes, icons, components
UPDATE_SNAPSHOTS=1 cargo test -p apertrust-ui --test gallery   # accept changed snapshots; review them like golden images
```

- **Every binary** takes darktable-style debug domains on stderr: `-d gpu|raw|pipe|perf|memory|golden|settings|control|event` (`signal` an alias), `-d all`, `-d verbose`; with one on, a banner comes first. `--version` (`-V`) prints the version and commit, libraries, GPU backends and where to report bugs (decision 44).
- **Settings.** `--user-data-dir DIR` (or `--config-dir`, `--configdir`, `APERTRUST_USER_DATA_DIR`; else the platform's folder) holds `settings.toml`; the CLI only reads it. `--conf KEY=VALUE` sets one for this run only, as darktable's does (decision 43). The rules for both are in [docs/crates.md](docs/crates.md) (`apertrust-cli`, and the app's settings file).
- **`edit`** changes `FILE.apxmp` beside the raw, which `export` and `preview` start from. **`export`** follows darktable-cli's size flags, format and file-on-disk rules (decision 56).
- **`compare-cpu.sh`** times darktable-cli against Apertrust on one machine into `target/compare-cpu/results.md` ([docs/crates.md](docs/crates.md), `apertrust-golden`).
- **Snapshots** let GPUs differ by 2/255 a channel and 2 pixels more (`kittest.toml`), so one made on any machine passes on the others; `crates/ui/tests/tolerance.rs` keeps it catching a token 3/255 off, a point's move, a lost letter.

**Versions** come from git tags (decision 44), not `Cargo.toml`: `git tag release-X.Y.Z` on a commit of main builds as `X.Y.Z`, later commits as `X.Y.Z+N~gHASH`. The full rules are in [docs/crates.md](docs/crates.md), `apertrust-base`.

**The raw corpus.** `apertrust-raw` decodes with rawler, a decoder in Rust (decision 23). 40 raw.pixls.us samples pin its output, so updating rawler is deliberate. Fixes pending upstream: Sony's lossy ARWs seed their dither from 16 bits where rawspeed uses 24 (the A1 and A6400 fail the exit test until then); the Nikon Z 9's white point is 15520 where the Z 8 and later have 15892; the Samsung NX1's black level is 0 where the data floors near 128.

```sh
tools/fetch-raw-corpus.sh                                   # ~1.8 GB into test-images/raw.pixls.us
cargo test --release -p apertrust-raw -- --ignored             # compare with tests/corpus/expected.txt
APERTRUST_BLESS=1 cargo test --release -p apertrust-raw -- --ignored   # accept a change, then review the diff
```

**CI** runs in three layers (decision 70). `ci.yml`, on every pull request and push to main: lint, cargo-deny, the tests on Linux (lavapipe) and macOS (Metal), and clippy on Windows and macOS. Windows' tests (WARP) are off for now: they ran past two hours. A pull request labelled `full-ci` runs every check even when it changes only docs; a label starts no run, so re-run the jobs after adding it. `nightly-tests.yml` runs them on Linux and macOS every night, opening an issue labelled `nightly-tests-failed` when they fail and closing it when they pass; both call `tests.yml`. `tools/check-windows-paths.sh`, in lint, fails when a file with a platform cfg is not in the list; add it there. A pull request whose every file is in `.github/docs-only-paths.txt` skips lint, clippy and the tests, and `tools/check-docs-paths.sh` fails when code reads a file that list matches. macOS is tested locally on every change. Every job has a timeout, and a new push cancels a pull request's run in progress. `timing.yml`, run by hand, times a 61 MP export and exposure changes on lavapipe and WARP. `nightly-builds.yml`, each night main has changed, builds Apertrust Dev's packages on each platform and publishes them as the `nightly` pre-release, moving its tag ([docs/packaging.md](docs/packaging.md#nightly-builds)). `codeql.yml`, `scorecard.yml` (OpenSSF), `dco.yml` (fails a pull request with a commit lacking its author's `Signed-off-by:`) and `sonarcloud.yml` (coverage and clippy, once the `SONAR_TOKEN` secret exists) run too. Actions are pinned by commit SHA, the version in a comment; Dependabot updates cargo and the actions weekly. Issue forms and the pull request template are in `.github/`.

### Golden-image harness

It runs locally only. It needs darktable-cli (`DARKTABLE_CLI`, `PATH` or the default install location) and darktable's integration-test images; `pipeline` needs the raw.pixls.us corpus (`test-images/raw.pixls.us` or `APERTRUST_RAW_CORPUS`, names `<Make>/<Model>/<file>`). A port passes when its kernel's output is within the case's ΔE2000 tolerance of darktable's; `gradient` checks the Slang derivatives against finite differences and forward against reverse mode. The `golden-tests` skill explains the rest.

```sh
export APERTRUST_TEST_IMAGES=~/Developer/darktable/src/tests/integration/images
cargo run --release -p apertrust-golden -- list
cargo run --release -p apertrust-golden -- render [CASE...]             # cached in target/golden
cargo run --release -p apertrust-golden -- run [CASE...] [--heatmaps DIR]   # run our kernels, check, time
cargo run --release -p apertrust-golden -- gradient [CASE...] [--explain] [--synthetic]
cargo run --release -p apertrust-golden -- raw-passthrough [IMAGE...]   # darktable vs rawler, no port needed
cargo run --release -p apertrust-golden -- raw-run [--as-shot] [IMAGE...]   # the raw stage and colorin vs darktable
cargo run --release -p apertrust-golden -- raw-defaults [IMAGE...]      # darktable's rawprepare defaults vs ours
cargo run --release -p apertrust-golden -- pipeline [IMAGE...]          # Phase 1's exit test
cargo run --release -p apertrust-golden -- pipeline IMAGE... --width W --height H [--hq BOOL --darktable-hq BOOL --upscale BOOL] [--add CASE]
cargo run --release -p apertrust-golden -- export RAW -o OUT.png [--darktable-xmp FILE] [--add CASE]... [--width W --height H --hq BOOL]
cargo run --release -p apertrust-golden -- compare EXPECTED.pfm ACTUAL.pfm --case NAME --heatmap diff.png
cargo test --release -p apertrust-golden -- --ignored            # end-to-end check of the harness
```

- `pipeline` with a size, `--add` and `export` are described in [docs/crates.md](docs/crates.md), `apertrust-golden`.

### Kernels

Each operation's kernel lives in its own crate, `crates/op/<module>` (decision 20), written in Slang for its autodiff (decision 9) and compiled to WGSL that is committed. Shared colour maths is in `crates/color`; the pass header and `components.slang` (`add_at`, `set_at`; decision 10) in `crates/plugin`. Every differentiable operation's kernel exports `<module>_backward` and `<module>_forward_diff`; the raw stage's kernels have none (decision 21). The `slang-kernels` skill holds the interface, the structure, the rules and Slang's derivative traps (`pow0`, not `pow`, among them).

```sh
tools/fetch-slang.sh             # pinned slangc into external/ (git-ignored), checksum-verified
tools/build-kernels.sh           # slang/*.slang -> generated/*.wgsl; commit the output
tools/build-kernels.sh --check   # lists stale or orphaned generated files; changes nothing
cargo run --release -p apertrust-golden -- run CASE --repeat 32   # speed comparison
WGPU_POWER_PREF=low cargo run --release -p apertrust-golden -- run CASE   # on an integrated GPU
```

## Crates

What each crate has built and planned is in [docs/crates.md](docs/crates.md).

| Crate | Role |
| --- | --- |
| `apertrust-engine` | The GPU-resident pipeline and the editing session; no UI dependencies |
| `apertrust-cli` | The headless binary: `gpu`, `info`, `edit`, `export`, `preview`, `library scan`; `batch`, `serve` and `mcp` later |
| `apertrust-golden` | The golden-image harness against darktable-cli, and Phase 1's exit test |
| `apertrust-raw` | Raw decoding via rawler, metadata as darktable reads it |
| `apertrust` | The desktop app (egui), in `crates/app` |
| `apertrust-ui` | The design system (decision 36): tokens, themes, fonts, icons, translation, components; no engine or module types; `apertrust-base` for `settings::Choice` alone |
| `apertrust-display` | The window's platform code, FFI that may use `unsafe`: display colour, the window frame and menu bar where winit and muda cannot (decision 39), native popups (decision 40), modal dialogs and alerts (decision 41), and the Windows app's terminal, attached to at its start (`docs/packaging.md`, Windows) |
| `apertrust-op-<id>` | One operation each, in `crates/op/<id>`: Slang kernel, generated WGSL, CPU-side setup, manifest and schema (decision 20) |
| `apertrust-operations` | The list of built-in operations, in darktable's order |
| `apertrust-base` | What every program sets up as it starts, nothing about images, colour, the GPU or the UI (decision 43): logging, version, folders, settings, the safe write, darktable's control jobs |
| `apertrust-plugin` | The operation interface |
| `apertrust-color` | Colour maths shared by operations, and numerical helpers two ports share (decision 65): Slang for kernels, Rust for setup |
| `apertrust-modules` | Modules (decision 28): what the user edits with, bound to operation parameters; no egui, wgpu, engine or operation crate |
| `apertrust-export` | What an export writes and where, with no GPU (decision 56): settings, size, variables, file-on-disk rules, JPEG and PNG |
| `apertrust-library` | Folder mode, libraries and projects, with no GPU (decisions 46, 47 and 52 to 55); never depends on the engine, modules, plugin or wgpu |
| `apertrust-fs` | Planned, only if bulk listing needs FFI that `rustix` and the `windows` crate do not cover (decision 65) |
| `apertrust-protocol`, `apertrust-server`, `apertrust-mcp` | Session protocol; the server, sync then the engine for browsers (decision 71); MCP; later phases |

- Each crate lives in `crates/<name>`, its package name without `apertrust-`: the app in `crates/app`, operation crates in `crates/op/<id>` (decision 65). A new crate goes into `members` in the root `Cargo.toml`.
- Add a crate when its first real code lands, and only when a module of an existing crate cannot do: `unsafe` FFI, a dependency direction that would otherwise break, or a decided boundary such as an operation's (decision 65). Keep the number of crates small.
- Shared dependency versions live in the root `Cargo.toml` under `[workspace.dependencies]`.
- Data files live in the top-level `data/`, as darktable's (decision 45): darktable's verbatim under their names, and those several crates or readers outside Rust read, the settings schema first. Each is added with its first consumer, its provenance in `data/README.md`, embedded at build time by the one crate that parses it, never read from an installed folder. Themes, icons, fonts and locales stay in `apertrust-ui`, kernels in their operation crates.

## Engineering rules

These come from the architecture doc and are not up for local reinterpretation.

- **Engine/UI separation.** The engine never depends on UI code. Every client (desktop UI, web UI, MCP, scripts) reaches it through the session protocol. Widgets only emit commands; they never touch engine state.
- **GPU-resident pipeline.**
  - Upload the raw once and keep intermediates on the GPU, with no host round trip between operations.
  - Storage buffers by default; textures only where hardware sampling helps (lens correction, rotation, resampling). Geometry interpolates in its kernel with darktable's interpolators, never by hardware filtering, whose precision differs between GPUs (decision 16).
  - Every operation has a GPU path; a CPU version is only a test reference. Without a hardware GPU the same kernels run on lavapipe (Linux) or WARP (Windows); there is no CPU code path (decision 7).
  - The engine submits only through `GpuContext::submit`, never `queue.submit`, taking turns with a window reconfiguring its surface on the same device (decision 18).
- **Kernels.**
  - fp32 only: redesign fp64 maths rather than translating it.
  - Never rely on NaN or infinity checks (WGSL may assume they never occur); sanitize inputs at operation boundaries.
  - Never write a local vector component or array element at an index known only at run time: FXC, the Direct3D 12 compiler the engine pins, fails on the vector component, and the array element, like a vector component in workgroup memory, is refused until tested on Windows. Use `add_at` and `set_at` from `crates/plugin/slang/components.slang`; the engine's naga test checks every kernel (decision 10).
  - Keep kernels differentiable-friendly: continuous parameters, no unnecessary hard clamps, thresholds or branches.
  - Generated WGSL is committed and validated with naga.
- **Modules and operations.**
  - A module is what the user sees; an operation is one engine algorithm, usually a darktable port (decision 4). Operations never appear in the UI.
  - Each operation parameter has exactly one owning module. Depth comes from inside that module, its tabs and collapsible groups, never from a second module; there is no Advanced controls toggle in the first pass (decision 29).
  - In the first pass a module is one darktable module, its parameters the controls under darktable's labels. White balance (over `temperature`) and Colour calibration (with its CAT tab) are both darktable's (decision 27); tone mappers are modules of their own, one per edit (decision 2; `docs/modules/README.md`, `tool-bindings`).
  - A module that is off is a no-op, covered by an invariant test (decision 32). A module darktable leaves off on a new photo is off until any of its controls changes, then on with darktable's defaults for the rest; its reset turns it off. A module whose operation every photo runs is untouched at its defaults.
- **darktable is the migration's reference, never a dependency** (decision 2).
  - Never load or link darktable modules.
  - The migration ends once, for the whole project, when every first-pass module is in the app and passes its comparisons (end of Phase 3). After it, Apertrust evolves on its own and claims no dependency on darktable; ported files keep darktable's copyright and provenance.
  - Port each module the same way: a short spec in `docs/ports/<module>.md` from darktable's source, noting deliberate differences; the kernel in Slang with its derivative entry points (none in the raw stage); the golden-image test (for the raw stage, `raw-run` and `raw-defaults`) and the gradient check (or the understood gap recorded in the spec); provenance (darktable module and source revision) in the manifest.
  - **The reference is darktable's latest stable release, currently 5.6.2** (decision 3): port from the source at its tag, `release-` and the number, not a newer checkout. The number lives in `REFERENCE_VERSION` (`crates/golden/src/darktable.rs`, read by the harness, `tools/compare-cpu.sh` and the `port-module` skill's `$DT_TAG`) and in each operation manifest's provenance. Prose, commands and comments say "the reference release". Masks and retouch, in Phase 4 after the migration, are ported from darktable's master at a commit their port spec names (decision 74).
  - To move to a new stable release: re-render the golden references with its darktable-cli, review each ported module's upstream changes and follow them (`port-module`, "Following an upstream darktable change"), then update the constant and each port's provenance.
- **Correctness.** Image quality is judged by golden-image ΔE2000 tests with per-module tolerances, never by eye: during the migration against darktable-cli at the reference release; after it against Apertrust's own renders taken at the commit that completes the migration, each change to a baseline a deliberate, reviewed decision for the log (decision 2).
- **Rust, not C or C++** (decision 5). No C or C++ library is compiled into Apertrust or shipped with it unless no Rust option exists; ONNX Runtime is the exception, loaded at run time through its C API. Libraries the platform provides and Apertrust never ships, loaded at run time, are outside the rule: the Vulkan loader and drivers (wgpu), libwayland, libxkbcommon and the X11 libraries (winit), libdbus (rfd), the OS's own frameworks and system DLLs. A library Apertrust would bundle, download or compile in needs its own exception, which the maintainer grants.
- **unsafe** is denied workspace-wide. Only FFI crates may opt out, with `#![allow(unsafe_code)]` at the crate root.
- **Dependencies** must pass `cargo deny check`: every licence GPLv3-compatible. Bundled fonts and Lucide's icons are the other licences: fonts OFL-1.1, the files unmodified and shipped with their licence text; icons derived from Lucide ISC, those from Lucide's Feather-derived icons MIT, each file naming its source, Lucide's notices in `crates/ui/assets/icons/LICENSE-Lucide` and `CREDITS.md` (decisions 36 and 64).

## Licence headers

Apertrust is GPLv3. Every source file (`.rs`, `.wgsl`, `.slang`, `.sh`) starts with this header, from the [GPLv3 header template](https://github.com/licenses/license-templates/blob/master/templates/gpl3-header.txt), as `//` comments (`#` after the shebang in shell scripts), followed by a blank line:

```text
Apertrust, a free, non-destructive, cross-platform raw photo editor
Copyright (C) 2026  Apertrust developers

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
```

A file that ports darktable code keeps darktable's copyright as a third line, with the years from the darktable source file it ports, e.g. `Copyright (C) 2009-2026  darktable developers (ported from src/iop/exposure.c)`. Generated WGSL gets its Slang source's header from `tools/build-kernels.sh`. `tools/check-license-headers.sh` checks every source file and runs in CI. Markdown documents have no header.

## Writing rules

- Never name commercial raw editors anywhere in the repo, including commit messages. Say "commercial raw editors" instead. (darktable's own data files, such as the XMP namespaces in its sidecars, stay as darktable writes them.)
- Documents state what is decided and built now: no dates of decisions, no who decided, no board cards, no earlier designs. Write short paragraphs, and use lists, tables and headings; add a Mermaid diagram where a flow or structure is easier seen than read.
- Commit messages and pull request descriptions are short and plain, written as a person would write them: the style is in `/draft-commit` and `/draft-pr`.
- Every commit carries a human's `Signed-off-by:` (`git commit -s`), the DCO (decision 67). An agent never signs off in an AI's name; it adds the maintainer's sign-off only to a commit it makes for him. An `Assisted-by:` or `Co-Authored-By:` trailer is allowed but never required: AI help is disclosed once, in the README and the AI policy, and every contribution is judged by the same rules.
- Test photos and AI models are downloaded, never committed (see `.gitignore`); local test raws go in `test-images/`, which git ignores. Force-add a small fixture only when a test needs it.
