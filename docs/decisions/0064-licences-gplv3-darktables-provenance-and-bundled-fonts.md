# 0064. Licences: GPLv3, darktable's provenance and bundled fonts

## Context

Apertrust ports darktable's code, which is GPL-3.0-or-later, so Apertrust is GPL too. What does that require of every file, every dependency and everything bundled, and how is it checked?

## Decision

**Apertrust is GPL-3.0-or-later** (`LICENSE`, the workspace's `license` in `Cargo.toml`).

- **Every source file** (`.rs`, `.wgsl`, `.slang`, `.sh`) starts with the GPLv3 header template's text, "Apertrust, a free, non-destructive, cross-platform raw photo editor" and "Copyright (C) 2026  Apertrust developers", as `//` comments (`#` after a shell script's shebang), then a blank line. Generated WGSL takes its Slang source's header from `tools/build-kernels.sh`. `tools/check-license-headers.sh` checks every file in CI.
- **A file that ports darktable code keeps darktable's copyright** as a third line, with the years from the darktable file it ports: `Copyright (C) 2009-2026  darktable developers (ported from src/iop/exposure.c)`. Each operation's manifest records its provenance, the darktable module and the release its port was written from ([0003](0003-the-reference-is-darktables-latest-stable-release.md)). The copyright lines and provenance stay after the migration ends ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).
- **darktable's data files** are copied verbatim under darktable's GPL, their provenance in `data/README.md` ([0045](0045-a-top-level-data-folder.md)).
- **Every dependency's licence is GPLv3-compatible,** checked by `cargo deny check` against `deny.toml`'s list in CI, run before a dependency is added ([dependencies.md](../dependencies.md)). jpeg-encoder's forward DCT is under the IJG licence, allowed: the FSF lists it as GPL-compatible, and `CREDITS.md` says the binary is based in part on the Independent JPEG Group's work.
- **`CREDITS.md`** lists every component from `Cargo.lock`, kept up to date by `tools/credits.sh` (its `--check` runs in CI), and About shows it.
- **Bundled fonts and Lucide's icons are the other licences:**
  - IBM Plex Sans under the SIL Open Font License 1.1, which lets fonts be bundled with any software, shipped unmodified, since "Plex" is a Reserved Font Name, with their licence text (`crates/ui/assets/fonts/OFL.txt`).
  - Most of the interface's icons are derived from [Lucide](https://lucide.dev)'s, under the ISC licence, © Lucide Icons and Contributors, some of them from Lucide's icons derived from Feather, under the MIT licence, © 2013-present Cole Bemis; both GPLv3-compatible. They are Apertrust's own files under its names in `crates/ui/assets/icons`, adapted where needed (the zoom magnifiers mirrored); each one's first line names its Lucide source and licence, `LICENSE-Lucide` beside them is Lucide's licence with both notices, verbatim, and `CREDITS.md`, which About shows, carries both notices. The icons drawn for Apertrust in Lucide's style carry no Lucide notice and are under the GPL.
  - The crate that ships them, `apertrust-ui`, declares `GPL-3.0-or-later AND OFL-1.1 AND ISC AND MIT`. No crate is added, so `cargo deny` is unaffected.
- **No darktable code is loaded or linked;** plugins keep their own provenance ([0020](0020-operations-as-native-plugins-one-crate-each.md)).
- **The websites' content** is CC BY-SA 4.0 and their code MIT, the name and logo excluded ([0067](0067-going-public.md)).

## Options considered

- A clean-room permissive licence: darktable's algorithms could not be ported from its source.
- A licence exception in `deny.toml` for egui's default fonts: a typeface the design does not use.
- Copyright lines dropped once a port is rewritten further: the GPL requires them for code written from darktable's source.

## Consequences

- A new source file needs its header before CI passes; a port its third copyright line and its manifest's provenance.
- A dependency whose licence is not on the list needs a decision first.
- A new icon derived from Lucide names its source and licence in its first line; if Lucide lists it as Feather's, the line says MIT and `CREDITS.md` names it.
