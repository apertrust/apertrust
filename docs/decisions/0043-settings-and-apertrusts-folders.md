# 0043. Settings and Apertrust's folders

## Context

Settings are read by the app, the engine, the export and later the server, the web client and MCP. Each needs one declaration with its kind, range and default, which a dialog can draw and the CLI can read. Apertrust also needs separate folders for separate setups (a development build beside an installed one), as darktable's `--configdir` gives.

## Decision

**One settings schema in TOML,** `data/settings.schema.toml` ([0045](0045-a-top-level-data-folder.md)), as darktable declares its settings in `darktableconfig.xml.in`. Each table is one setting, its header the permanent dotted id of its key in `settings.toml` (`scale`, `side_panel.width`, `engine.memory_budget`), with:

- `owner`: `app`, `engine` (every process that runs the engine), `export` (the Export dialog's settings, [0056](0056-export-as-an-engine-job.md)) or `module:<id>` (a module's tool setting);
- `kind` and its fields: boolean, number (`min` or `above`, `max`, `step`, `decimals`, `unit`, `factor`), choice, text, list, or a table keyed by another registry's ids; whole numbers and folders come with the first setting that needs one;
- `default`, or `"computed"` in Rust, or `optional = true` for a value only its owner keeps;
- `page` and `group` in the Settings dialog, `applies` (`at-once`, `next-photo`, `after-restart`) and `platforms`;
- a `#` comment above it, a developer's note.

A choice names its Rust type (`type = "apertrust_ui::Builtin"`) instead of listing options; the options come from the type through `apertrust_base::settings::Choice`, each value's stable id. Words stay in Fluent, by ids that follow from the setting's id.

**Each owner's crate generates typed statics** from the schema in its `build.rs`, the settings registry (`settings.get(&SCALE)` checked by the compiler). The generator, `apertrust_base::settings::codegen`, reads the schema with toml_edit in the file's order and refuses unknown fields, wrong kinds, duplicate ids, defaults out of range and missing defaults, naming the line; a choice's default that none of its type's ids match fails where the static compiles.

**The store,** `apertrust_base::settings::Store`, opened on one or more owners' registries: typed `get`, `set` and `reset`. The file's rules: a missing key is its default; an unreadable value gets its default with a warning and stays in the file until changed; a number out of range reads as the nearer end; unknown keys stay; a damaged file is kept as `.bak`; a newer version's file is never overwritten. Writes go on a thread of their own a second after the last change and on exit, merging only what changed into the file as it is, so two instances share a folder per key. `Store::read_only` never writes. Versions rise for a removal or a change an older build would misread, never for an addition.

**`--conf KEY=VALUE`** (repeatable, the last for a key counting) sets a setting for one run, read by its kind with the file's rules, held over the file and never written; an unknown key or an unreadable value stops the start; a set of an overridden key is refused, and the dialog draws its row disabled.

**The user data folder** is chosen by `--user-data-dir` (`--config-dir` and darktable's `--configdir` accepted), then `APERTRUST_USER_DATA_DIR`, then the platform's: `~/Library/Application Support/Apertrust`, `%APPDATA%\Apertrust\config`, `$XDG_CONFIG_HOME/apertrust`. A leading `~` is the home folder; a relative path is from the current folder. The app creates a missing one; one given by flag or variable that cannot be created or written stops the start and never falls back. The CLI only reads (`Folders::find_to_read`), creating nothing. It holds `settings.toml`, `state.toml` (Open Recent), themes, presets (`presets/export.toml`), later styles, scripts and reports. It never holds a library, and it never decides where edits go ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md)).

**Planned:** the cache folder, the platform's by default and `<user data>/cache` when one is given, `--cache-dir` and `APERTRUST_CACHE_DIR` with the first cache; Apertrust Dev, a folder of its own for builds that are not releases, from the first release tag.

**The engine opens no folder or file:** the binaries hand it options (`apertrust_engine::settings::Options::read`); the golden harness reads no settings. **The Settings dialog** has nine pages, as darktable's preferences: General, Import, Library, Editing, Processing, Storage, Shortcuts, Presets and AI. Its rows are drawn from the schema, each setting with a `page` and a `group` a row there in the schema's order; Shortcuts, Storage, Presets and AI are panels built by hand; a page that holds nothing yet is listed dimmed and cannot be chosen. What the app remembers on its own is never shown.

**`apertrust-base`** holds what every program sets up as it starts and nothing about images, colour, the GPU or the UI: logging and the banner ([0044](0044-logging-the-banner-and-versions.md)), the version, folders, settings, the safe write (`fs`) and jobs ([0057](0057-jobs-as-darktables-control-jobs.md)), each behind a feature.

## Options considered

- Settings declared in Rust in each owner's crate: three places to look, unreadable without compiling.
- Options listed in the schema: each choice's type declared twice.
- serde on a struct: one bad field fails it, with no words or ranges.
- The machinery in the engine, the app or `apertrust-ui`: the CLI and build scripts could not use it.
- darktable's whole-file write on exit: two programs overwrite each other.
- "Config folder" as the term (the folder holds more than configuration); a folder split by XDG kind on Linux.
- An explicit folder that falls back to the platform's when unusable: a development setup silently writing into the installed one.

## Consequences

- A new setting is a table in the schema and its messages in three languages; tests hold ids, ranges and messages.
- `--conf` and the dialog work for every owner without code of their own.
