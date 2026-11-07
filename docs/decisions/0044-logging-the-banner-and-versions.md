# 0044. Logging, the banner and versions

## Context

How does Apertrust log, so that a person can measure performance, diagnose problems and read a pasted log, without a flood? darktable's `-d` domains and startup banner are known to its users. And where does a build's version come from?

## Decision

**darktable's model on `tracing`,** in `apertrust-base`'s `log` module, behind a `log` feature every binary turns on:

- silent but for warnings and errors, until `-d DOMAIN` switches a domain on, on stderr, repeatable, with `all` and `verbose`, in every binary;
- the domains: gpu, raw, pipe, perf, memory, golden, settings, control, event (`signal` an alias);
- library crates log with the domain as the target: `debug!` for a domain's lines, one per step; `trace!` for the detail `verbose` adds; `warn!` and `error!`, which always show; nothing at debug inside a per-pixel or per-frame loop;
- domains match targets exactly, so other crates' debug output never shows; `RUST_LOG` replaces the switches for developers;
- the CLI's stdout keeps only results; timings go to `-d perf`.

**A line** is the seconds since start, `[domain]`, then a short lowercase phrase, the fact first and details as `key: value`, within 100 columns (`WIDTH`) in the common case. A list is a summary line, with a line per item under `verbose`. A warning or error names a whole path; a debug line a file's name. Other crates' lines are put on one line and cut at the width with "…", keeping the innermost cause, whole under `verbose`; an error is never cut. A line repeated with the same target, level and text is written once, then `(repeated N×)`. rawler's lens lookups are one line under `-d raw`.

**The startup banner,** as darktable's, prints only while a domain is on or `RUST_LOG` is set: the program and version with the commit (`Apertrust CLI 0.1.0+138~gdf3e921 [macos arm64 release]`), the copyright and licence, a `Build:` list of what the binary has (wgpu and its backends, eframe and egui, rawler, the Slang kernels with naga, AccessKit, the languages, ONNX Runtime, and for the harness the darktable-cli it found), the documentation and issue links, then `[starting] as:` and the command line. `--version` (`-V`) prints it too; About shows the version alone.

**Versions come from git tags.** A release is tagged `release-X.Y.Z` on a commit of main and builds as `X.Y.Z`; N commits later `X.Y.Z+N~gHASH`; before any release tag `0.0.0+N~gHASH`, N counting every commit; `-dirty` after it when tracked files had changes as HEAD last moved. Other tags are ignored. A malformed release tag, a shallow clone or no git builds as `0.0.0` with a warning saying why. A packager's `APERTRUST_VERSION` is used as given. `Cargo.toml`'s version is not the app's. Each binary's own `build.rs` computes it (`apertrust_base::program!()`, behind the `build` feature), reading `Cargo.lock` and the Slang pin, and runs again only when HEAD, a tag or `shallow` changes, so a commit rebuilds no library. `git status --porcelain` finds changes without taking the index's lock.

## Options considered

- tracing's default format with module targets: levels, module paths and timestamps are noise to a person, and prefix matching lets other crates' debug output through.
- JSON lines: for machines; a server or MCP can add them later as a second layer.
- Levels without domains: no timings without everything else.
- The banner on every start: it ends the silence for the app opened from the desktop; only on request: a pasted log lacks it.
- Versions typed by hand: they drift. A build-information crate: new dependencies, some over C's libgit2.
- 80 columns, or the terminal's width, which a file or a pasted report does not have.

## Consequences

- Every binary calls the same initialisation and keeps its `Logging` until it ends.
- Planned: a log file and a crash report in the user data folder, starting with the banner. The crash report replaces the home folder with `~` and photo and folder names with placeholders, keeping file extensions and camera models; the local log file keeps full paths. Planned too: puffin for frame profiling.
