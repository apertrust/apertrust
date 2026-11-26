# 0070. CI in three layers

## Context

Apertrust runs on Windows, macOS and Linux ([0006](0006-two-programs-on-three-desktop-platforms.md)), and its tests need a GPU. GitHub's hosted runners have none, so the tests run on software GPUs ([0007](0007-a-software-gpu-instead-of-a-cpu-path.md)): lavapipe on Linux, WARP on Windows, the paravirtual Metal device on macOS. WARP is slow: the engine's tests alone take 30 minutes on it, a whole Windows run more than an hour. macOS takes 15 to 20 minutes. Running every platform's tests on every pull request makes each one wait an hour for Windows. Which tests run where, and when?

## Decision

**Three layers.**

1. **Every pull request and push to main** (`.github/workflows/ci.yml`): `pre-commit`, every check of `.pre-commit-config.yaml` on every file on one Linux runner (the formatting, pre-commit's general file checks, shellcheck, actionlint, typos, cargo-deny, the licence headers, the UI strings, the Windows paths' guard, the documentation paths, the credits, the generated WGSL, the API docs, and **clippy for Linux, Windows and macOS**, `tools/clippy.sh --all`, which compiles the tests and each platform's cfg code without linking; a build script is compiled for Linux, so its code for another platform is compiled there where `cfg!()` chooses it at run time, which needs its dependencies to build on every machine), and **the full test suite on Linux and macOS**, on lavapipe and on the runner's paravirtual Metal device. The checks are pre-commit's, so a contributor runs locally what CI runs.
2. **Windows' tests are off for now:** on WARP the suite ran past two hours even in release and one test at a time, and in parallel its Direct3D 12 devices crash; clippy on Windows still compiles the tests. When they run again, they run on a pull request that changes a Windows-relevant path, listed in `.github/windows-paths.txt`, one git glob a line, and on one labelled `full-ci`, which also runs every check on a pull request that changes only docs. The list is wider than the files with a platform cfg, since Windows also fails in code without one, on Unix paths in tests and CR LF in fixtures: the window's platform code and the packages; every crate or file that handles paths, files or folders; every file with a platform cfg; the manifests, the lockfile, the toolchain, `.cargo/`, `.gitattributes`, the workflows and `kittest.toml`. CI's `platforms` job matches the pull request's changes, from its merge base, against the list.
3. **Every night** (`.github/workflows/nightly-tests.yml`, 02:23 UTC, and by hand): the full test suite on Linux and macOS, and on Windows once its tests run again. A failed night opens an issue labelled `nightly-tests-failed`, naming the failed jobs and linking the run, or comments on the one already open; a night that passes closes it.

**A guard keeps the list current.** `tools/check-windows-paths.sh`, run by pre-commit in CI and locally, fails when a Rust source or Cargo manifest with a platform cfg (`cfg(windows)`, `cfg!(target_os = …)`, `cfg_attr(unix, …)`, inside `any()` or `not()` too) is not covered by the list. With `--changed REVS` it prints the files a diff changes that the list covers, which is how CI matches.

**A pull request that changes only documentation skips the code's checks:** when every file it changes is matched by `.github/docs-only-paths.txt` (`docs/` and the top-level Markdown, the issue forms and the pull request template, less `CREDITS.md`, which code reads; nothing under `docs/` is read by code, so every edit there skips them), the `platforms` job skips pre-commit's long checks, clippy and the API docs, and the tests, which report as skipped, and CodeQL's own first job skips its analysis by the same check, main's pushes and its weekly run analysing every change still; `tools/check-docs-paths.sh`, in pre-commit, fails when a file the list matches is named by code in `crates/` or `tools/`, and main and a pull request labelled `full-ci` always run everything.

**One check is required of CI:** `ci-passed`, the last job of `ci.yml`, passes when every other job passed or was skipped and fails naming those that failed or were cancelled. The ruleset on main requires it and the DCO workflow's `signoff` alone, since a skipped matrix job or reusable workflow reports under another name than its runs.

**Every job has a timeout**: tests 40 minutes on Linux and macOS, the rest what they need. CI, CodeQL and the DCO check cancel a pull request's run in progress when a new push arrives; main's runs all finish. The tests are one reusable workflow, `.github/workflows/tests.yml`, which CI and the nightly call with their runners, so both test the same way.

## Options considered

- **The full suite on every platform for every pull request:** each pull request waits more than an hour for WARP, for bugs that are rare between nights.
- **Building only, with no tests on Windows or macOS:** a platform's bug found only by a user, and FXC, which only the Windows tests run ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md)), never run.
- **GPU runners:** paid, and the software GPUs are what the tests must pass on anyway ([0007](0007-a-software-gpu-instead-of-a-cpu-path.md)).
- **A list of only the files with a platform cfg:** misses the Windows failures in files without one.
- **CI's own list of checks beside pre-commit's:** two lists of the same checks, which drift apart.
- **Clippy on Windows and macOS runners:** two more jobs, one on a costlier runner, for what clippy checks from Linux too, since it never links, a build script's code for Windows included, since it is chosen at run time.
- **A path filter action:** a third-party action for what `git diff` and one list do.

## Consequences

- While Windows' tests are off, a Windows-only bug, or a kernel FXC refuses, can reach main and shows only on a Windows machine. The engine's naga test still catches a kernel's write at a run-time index on every machine ([0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md)).
- A build script's code for Windows, the app's resources in `crates/app/build.rs`, is checked by clippy on Linux but first run, with the SDK's `rc.exe`, by the nightly packages' Windows build. A build script's platform code behind `#[cfg]`, which a dependency that builds on one platform alone needs, is first compiled on that platform.
- A file that gains a platform cfg fails `pre-commit` until it is added to the list.
- A contributor who wants every check on a pull request that changes only docs asks for the `full-ci` label ([CONTRIBUTING.md](../../CONTRIBUTING.md)). A label starts no run of its own, since such a run would cancel the one in progress: after adding `full-ci`, re-running the jobs picks it up, as CI reads the pull request's labels when it runs.
- The `full-ci` and `nightly-tests-failed` labels exist in the repository. `nightly` is kept for the nightly builds, planned ([packaging.md](../packaging.md)).
