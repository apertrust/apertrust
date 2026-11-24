# Data

The data files Apertrust is built with (architecture decision 45), kept as darktable keeps its own in its `data/`. Two kinds live here:

- darktable's files, under darktable's names and unchanged;
- files that several crates, or readers outside Rust, read;
- the built-in themes, `themes/`, Apertrust's own design data (decision 36).

Each file arrives with its first consumer, never before. It is embedded at build time, with `include_str!` or by a build script, by the one crate that parses it; other crates reach what it parsed through that crate. Nothing is read from an installed folder at run time. Icons, fonts and locales stay in `apertrust-ui` (decision 36).

| File | Provenance | Licence | Read by |
| --- | --- | --- | --- |
| `settings.schema.toml` | Apertrust's own: every setting declared once (decision 43) | GPL-3.0-or-later | `apertrust-base`'s generator, `settings::codegen`, through the app's, the engine's and `apertrust-export`'s `build.rs` |
| `themes/dark.toml`, `grey.toml`, `light.toml`, `high-contrast.toml` | Apertrust's own: the built-in themes' tokens (decision 36) | GPL-3.0-or-later | `apertrust-ui`'s `theme.rs`, with `include_str!` |
| `workspaces/default.toml` | Apertrust's own: the Default workspace, built in, from decision 38's tabs and lists (decision 96), and the Toolbar's default list (decision 92); a test holds it to them | GPL-3.0-or-later | The app's `layout.rs`, embedded with `include_str!` |
| `workspaces/essentials.toml` | Apertrust's own: the Essentials workspace, built in: Photos, Adjust with the tools most edits need, History, every other tool hidden (decision 96), and the Toolbar's default list (decision 92); a test holds it to them | GPL-3.0-or-later | The app's `layout.rs`, embedded with `include_str!` |
| `presets/export/for-sharing.appreset` | Apertrust's own: Export's built-in For sharing, JPEG within 2048 × 2048 px in sRGB, the rest at darktable's defaults (decisions 90 and 106; darktable ships no export presets); a test holds it read and in the one layout | GPL-3.0-or-later | `apertrust-library`'s `presets`, embedded with `include_str!` |
| `presets/export/full-size-jpeg.appreset` | Apertrust's own: Export's built-in Full size JPEG, JPEG with no size limit in sRGB, the rest at darktable's defaults (decisions 90 and 106); a test holds it read and in the one layout | GPL-3.0-or-later | `apertrust-library`'s `presets`, embedded with `include_str!` |

A darktable file's row gives the darktable tag it was copied from, its git blob, its size in bytes and its SHA-256, which a test in the crate that embeds it holds, so a later release's file is a deliberate change.
