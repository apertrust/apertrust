# Contributing to Apertrust

Thank you for your interest. Apertrust is in early development, so the way to contribute is still small and simple. This page says how.

## Before you start

- **Open an issue or a discussion first** for a feature, a darktable port or any large change. It lets us agree on the scope before you spend time on code. A small fix or a typo needs no issue.
- **A new tool or a port:** [docs/contributing/new-processing-tool.md](docs/contributing/new-processing-tool.md) walks through one, from the operation's crate to the pull request.
- **Read the rules.** [AGENTS.md](AGENTS.md) holds the engineering rules and the commands. [docs/principles.md](docs/principles.md) says what Apertrust is for.
- **Design decisions** are recorded in [docs/decisions/](docs/decisions/README.md), a file each with its number. They are settled. To change one, open an issue that says which decision and why.

## Pull requests

- One small change per pull request, scoped to its issue. Link the issue.
- Say what changed and how you checked it, in your own words.
- Keep unrelated clean-ups for a pull request of their own.
- Split a pull request into one commit per separate piece of work (a command, its CLI, its docs), each building on its own, so the pull request is ready to merge as it stands.
- Fix review comments in the commit they belong to, not in a new commit on top: `git commit --fixup=<sha>`, then `git rebase -i --autosquash origin/main` (save the list unchanged; a stacked pull request names its parent branch instead of `origin/main`), and force-push with `--force-with-lease`.

## Checks

Run these before you open a pull request. CI runs the same, the tests on Linux and macOS:

```sh
pre-commit run --hook-stage manual --all-files   # every check but the tests
cargo test --workspace                           # the tests
```

[pre-commit](https://pre-commit.com) runs the checks in `.pre-commit-config.yaml` when you ask it to, each only when a file it reads changed. It is a Python program; install it once, with whichever you have:

```sh
pipx install pre-commit            # or: uv tool install pre-commit
brew install pre-commit            # macOS, Homebrew
sudo apt install pre-commit        # Debian, Ubuntu; other distributions package it too
```

Then, in the checkout:

```sh
pre-commit run                                         # the quick checks, on the staged files
pre-commit run --all-files                             # the quick checks, on every file
pre-commit run --hook-stage manual --all-files         # every check, the long ones too
pre-commit run clippy --hook-stage manual --all-files       # one long check
pre-commit run --hook-stage manual --from-ref main --to-ref HEAD   # every check, on this branch's changes
```

- **The quick checks:** the formatting; pre-commit's general file checks (large files, merge conflicts, private keys, the syntax of TOML, YAML, JSON and XML, line endings, trailing whitespace and more); shellcheck on the scripts, actionlint on the workflows, typos on the spelling (`typos.toml` holds the words it accepts) and cargo-deny once a manifest or `deny.toml` changes; and the repository checks. Each is given only the files it looks at. Those that fix what they find, as rustfmt does, fail once: stage their changes and run again. The kernels' check runs only when a Slang or generated file changed, and then compiles every kernel, about 25 seconds.
- **The tools:** pre-commit installs the ones it runs, at the versions the config pins, on its first run, which takes a few minutes. Rust, clippy and rustfmt are rustup's, from `rust-toolchain.toml`.
- **The long checks:** clippy (`clippy`, `tools/clippy.sh --all`: Linux, Windows and macOS from any machine, rustup adding each platform's standard library on the first run) and the API docs (`api-docs`) build the workspace and take minutes, so they run in the manual stage alone. Given only documentation, they skip it, as CI does; `--all-files` gives them every file, so they run, and `--from-ref main --to-ref HEAD` gives them only the branch's changes.
- **The tests** are not among them: run `cargo test --workspace` yourself.
- **As git hooks, if you want them:** nothing runs unless you ask. `pre-commit install` runs the quick checks before each commit; the long ones never run as a hook.
- **A kernel:** `tools/build-kernels.sh --check` needs the pinned Slang compiler, which `tools/fetch-slang.sh` downloads.
- **In CI:** every pull request runs pre-commit's checks on every file, as `pre-commit run --hook-stage manual --all-files` does, but for clippy and the API docs on a pull request that changes only documentation.
- **Windows and macOS:** a pull request that changes code, or is labelled `full-ci`, runs the tests on Linux and macOS, and clippy for Windows and macOS on Linux, as `tools/clippy.sh --all` does, which compiles their code and tests; a build script is compiled for Linux, so its code for another platform is checked there where `cfg!()` chooses it at run time, as the app's Windows resources are, which needs its dependencies to build on every machine; code behind `#[cfg]` is first compiled on its own platform. Windows' tests are off for now, since they take hours there. A file that gains a `cfg(windows)`, `cfg(unix)` or `cfg(target_os = …)` goes into that list; `tools/check-windows-paths.sh` says which are missing.
- **Required checks:** a pull request merges when `ci-passed`, which CI's other jobs feed, and `signoff`, the sign-off check, pass. A pull request that changes only documentation skips the code's checks, and `ci-passed` still passes.
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
