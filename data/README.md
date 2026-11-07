# Data

The data files Apertrust is built with (architecture decision 45), kept as darktable keeps its own in its `data/`. Two kinds live here:

- darktable's files, under darktable's names and unchanged;
- files that several crates, or readers outside Rust, read.

Each file arrives with its first consumer, never before. It is embedded at build time, with `include_str!` or by a build script, by the one crate that parses it; other crates reach what it parsed through that crate. Nothing is read from an installed folder at run time. Themes, icons, fonts and locales stay in `apertrust-ui` (decision 36).

| File | Provenance | Licence | Read by |
| --- | --- | --- | --- |
| `settings.schema.toml` | Apertrust's own: every setting declared once (decision 43) | GPL-3.0-or-later | `apertrust-base`'s generator, `settings::codegen`, through the app's, the engine's and `apertrust-export`'s `build.rs` |

A darktable file's row gives the darktable tag it was copied from, its git blob, its size in bytes and its SHA-256, which a test in the crate that embeds it holds, so a later release's file is a deliberate change.
