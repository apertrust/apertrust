# Contributing to Apertrust

Thank you for your interest. Apertrust is in early development, so the way to contribute is still small and simple. This page says how.

## Before you start

- **Open an issue or a discussion first** for a feature, a darktable port or any large change. It lets us agree on the scope before you spend time on code. A small fix or a typo needs no issue.
- **Read the rules.** [AGENTS.md](AGENTS.md) holds the engineering rules and the commands. [docs/principles.md](docs/principles.md) says what Apertrust is for.
- **Design decisions** are recorded in [docs/decisions/](docs/decisions/README.md), a file each with its number. They are settled. To change one, open an issue that says which decision and why.

## Pull requests

- One small change per pull request, scoped to its issue. Link the issue.
- Say what changed and how you checked it, in your own words.
- Keep unrelated clean-ups for a pull request of their own.

## Checks

Run these before you open a pull request. CI runs most of them, the tests on Linux:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
tools/check-license-headers.sh
tools/check-ui-strings.sh
tools/check-windows-paths.sh
tools/build-kernels.sh --check
tools/build-api-docs.sh
```

- **A kernel:** `tools/build-kernels.sh --check` needs the pinned Slang compiler, which `tools/fetch-slang.sh` downloads.
- **Windows and macOS:** a pull request runs clippy on both, which compiles their code and tests, and the tests on Windows when it changes a path listed in `.github/windows-paths.txt`. Every platform's tests run each night. If your change needs Windows or macOS tested before it is merged, ask for the `full-ci` label, which runs the tests on both. A file that gains a `cfg(windows)`, `cfg(unix)` or `cfg(target_os = …)` goes into that list; `tools/check-windows-paths.sh` says which are missing.
- **A new dependency:** run `cargo deny check`, and follow [docs/dependencies.md](docs/dependencies.md).
- **A kernel or a port:** CI cannot run darktable-cli, so run the golden checks locally and paste their results in the pull request: `apertrust-golden run CASE` for the ΔE2000 results and `apertrust-golden gradient CASE` for the derivatives. AGENTS.md's Golden-image harness section says what they need.

## Licence headers

Every source file (`.rs`, `.wgsl`, `.slang`, `.sh`) starts with the GPLv3 header given in AGENTS.md. A file that ports darktable code also keeps darktable's copyright line, with the years of the file it ports.

## Sign-off

Every commit carries your `Signed-off-by:` line, the [Developer Certificate of Origin](https://developercertificate.org/). It says you have the right to submit the change under the project's licence. Add it with:

```sh
git commit -s
```

The sign-off must be a person's name and address, never a tool's.

## Commit messages

Short and plain, as a person would write them. One line saying what the change does is often enough; add a few sentences when the reason is not obvious.

## Ports from darktable

Apertrust migrates darktable's processing module by module. A port:

- reads darktable's source at its latest stable release, the reference release, never a newer checkout;
- follows the `port-module` workflow: a short spec in `docs/ports/<module>.md` that notes every deliberate difference, the kernel in Slang, then the golden and gradient checks;
- records its provenance (darktable's module and source revision) in the operation's manifest;
- keeps darktable's copyright line in every file it ports.

## AI-assisted work

AI-assisted contributions are welcome under the same rules as any other; [AI_POLICY.md](AI_POLICY.md) says what that means. The agent kit, [apertrust/agent-kit](https://github.com/apertrust/agent-kit), holds the skills, agents and commands the project uses, and helps an AI-assisted contributor follow the rules.

## What reviewers look for

- The change does what its issue asks, and no more.
- It follows the engineering rules and the decisions it touches.
- The checks pass, and for a kernel or a port the golden results are within tolerance.
- The code is simple, and its author can explain it.

## Conduct

Everyone taking part follows the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues are reported privately, as [SECURITY.md](SECURITY.md) says.
