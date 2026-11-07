# 0045. A top-level data folder

## Context

Some files are data rather than code: the settings schema, which build scripts and readers outside Rust read, and darktable's data files (its white balance presets, its noise profiles), which more than one crate needs and which a later darktable release changes. Where do they live, and how does a binary get them?

## Decision

**A top-level `data/` folder, as darktable's, its files embedded at build time.**

- **What it holds:** darktable's data files verbatim under darktable's names, so `data/wb_presets.json` is darktable's `data/wb_presets.json` and a later release's file is compared path for path; and the data files several crates or readers outside Rust read. Today it holds `settings.schema.toml` ([0043](0043-settings-and-apertrusts-folders.md)).
- **Planned files:** `wb_presets.json`, darktable's white balance presets, parsed by `apertrust-raw` for the camera's presets in the image context ([0024](0024-the-cameras-data-in-the-image-context.md), [0027](0027-both-of-darktables-white-balances.md)); `noiseprofiles.json`, for Denoise (profiled). Each file comes with its first consumer, never before.
- **Provenance:** `data/README.md` gives each file's provenance, licence and readers, since JSON holds no comments; for a darktable file, the tag it was copied from, its git blob, its size and its SHA-256, under darktable's GPL-3.0-or-later. A test in the crate that embeds a darktable file holds its SHA-256, so a new release's file is a deliberate change.
- **Embedded once:** each file is embedded, with `include_str!` or by a build script, by the one crate that parses it. Other crates reach what it parsed through that crate or the image context, so no binary holds a file twice and no two parsers drift.
- **Never read at run time** from an installed folder, as darktable reads `share/darktable`: files beside the binary, which a distribution, a person or a second install can change, would render one edit differently on two computers with one build (principle 2), and `cargo run`, the tests and the golden harness need nothing installed. The cost is binary size, about 6.4 MB once both darktable files ship; compressing them waits until packaged size matters.
- **What stays elsewhere:** themes, icons, fonts and locales in `apertrust-ui`, which owns them ([0036](0036-the-design-system.md)); kernels in their operation crates ([0020](0020-operations-as-native-plugins-one-crate-each.md)); test fixtures and golden cases with their tests. darktable's JSON Schemas for its files are not copied, since nothing reads them.

## Options considered

- Each file in its consumer's crate: `wb_presets.json` has two consumers (the temperature operation's defaults and the White balance module, which cannot depend on each other), and the schema has readers outside Rust.
- A crate of `include_str!` constants: a crate without code that every layer depends on.
- Reading the files at run time from an installed folder: renders can differ between machines, and an install path is needed on three platforms before packaging.
- Compressing now: saves about 5 MB a binary, not yet worth the code.
- A CI script over the README's checksums in place of tests: it would run only on hosted CI.

## Consequences

- A port that needs a darktable data file copies it here at the reference release ([0003](0003-the-reference-is-darktables-latest-stable-release.md)) with its row and its checksum test.
- Moving to a new darktable release compares `data/` file by file.
