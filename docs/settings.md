# Settings and folders

How Apertrust's settings are declared, generated, stored and read, which folders it keeps them in, and how an edit is kept safe from another user data folder or another build (decision 43). What the desktop app keeps in its settings file and how it draws the Settings dialog are [gui.md](gui.md#settings-and-workspace)'s; the crates' state is in [crates.md](crates.md).

**Status.** Built: the schema with the boolean, number, choice, text, list and table kinds, a number's unit and factor, and each setting's page and group, its generator and every owner's registry; the store with the file's rules and the merge-write; `--conf`; the user data folder with its flags and variable; development builds' own folder, Apertrust Dev's; edits a newer Apertrust wrote opened for viewing only; the Settings dialog's pages of rows. Planned: whole numbers, folders and platforms in the schema, each with the first setting that needs one; the cache folder, with the thumbnails; the Settings dialog's hand-built pages, Storage and Shortcuts first.

## The settings schema

Every setting is declared once, in one schema file in TOML, `data/settings.schema.toml` (decision 45), as darktable declares its settings in `data/darktableconfig.xml.in`. Each table is one setting, for example:

```toml
# The interface's size, on top of the system's scale.
[scale]
owner = "app"
kind = "number"
min = 0.5
max = 3.0
step = 0.1
decimals = 2
default = 1.0
applies = "at-once"

[theme]
owner = "app"
kind = "choice"
type = "apertrust_ui::Builtin"
default = "dark"
applies = "at-once"
```

A setting holds:

- **its id,** the table's header: the dotted path of its key in `settings.toml`, permanent, its parts snake\_case (`scale`, `side_panel.width`, `engine.memory_budget`, `tools.exposure.lightness`);
- **`owner`:**
  - `app`: read by the desktop app alone;
  - `engine`: read by every process that runs the engine, the app, the CLI and later the server;
  - `export`: the Export dialog's last settings (decision 56), kept by the app and read by the CLI's `export`;
  - `module:<id>`: a module's tool setting (decision 29), kept by the client and sent with the command that runs the tool;
- **`kind`** and its fields:
  - boolean;
  - number: `min`, `max`, `step`, `decimals` (how many it is kept and shown with), `unit`, the id of the words it is shown with (`unit-<unit>`, as `mib`), and `factor`, what it is multiplied by where it is shown, as darktable's slider factor (`scale`, kept as 1.1, shows as 110 %); or, in place of `min`, `above`, what it must be more than, as a size's parts must be more than 0;
  - whole number, choice (`type`), text, folder;
  - list: `of`, its items' kind, and `length` when it is set;
  - table keyed by another registry's ids: `by` (`actions`, `sections`, `modules` or `groups`) and `of`, its values' kind, as `[edit.groups]`'s `by = "modules"` and `of = { kind = "table", by = "groups", of = "boolean" }`. An item's or a value's kind is a kind's name or an inline table of `kind` and that kind's fields;
- **`default`:** a value, or `"computed"` for one computed in Rust at start (the language follows the system's). A setting with no value until its owner keeps one, as the window's position, has none and says **`optional = true`**, which makes its static's value an `Option`. Any other setting without a default is refused;
- **`page`**, one of the Settings dialog's nine, `general`, `import`, `library`, `editing`, `processing`, `storage`, `shortcuts`, `presets` or `ai` ([The Settings dialog](#the-settings-dialog)), and **`group`** on it, snake\_case, each with the other; or neither, for what the app keeps by itself (the window's placement, the sections open, the scopes' view, the grid panel's layout, the page Settings last showed). A page's groups and rows are in the schema's order;
- **`applies`:** `at-once`, `next-photo` or `after-restart`;
- **`platforms`,** as an action's;
- **a `#` comment** above it: a developer's note, as darktable's longdescription, which its static's doc comment carries.

**Built so far:** the boolean, number, choice, text, list and table kinds; `default`, `optional`, `owner`, `applies`, `page` and `group`; a number's `above`, for the window's size and scale factor, and its `unit` and `factor`, for the interface's size and the memory budget; and each owner's `ALL`, every setting it owns, the registry its store reads. Whole numbers, folders and `platforms` come with the first setting that needs one.

**Words** stay in Fluent, by ids that follow from the id, as a module's do: `setting-<id>-label`, `-full`, `-promise` and `-option-<option>`, the id's dots and underscores as hyphens, with `settings-page-<page>` and `settings-group-<group>`. A tool setting keeps its module's, `module-<module>-setting-<setting>-…` (decision 29). The schema holds no English.

**How it becomes code:**

- **A choice names its Rust type** instead of listing its options, so no type is declared twice. The options come from the type through `Choice`, a small trait in `apertrust-base`: each value's stable id and every value in the dialog's order, as associated constants. The generated code checks the default against them in a constant, so a default no value has fails the build. A computed default calls a function the owner's crate defines under the setting's name, so a missing one fails the build too.
- **Each owner's crate generates its own settings** in its `build.rs`: the app's for `owner = "app"`, the engine's for `engine`, `apertrust-export`'s for `export`, the modules' for `module:<id>`. They are typed statics written into `OUT_DIR`, not committed:
  - `settings.get(&SCALE)` and `settings.set(&SCALE, value)` are checked by the compiler;
  - the engine's generated code never sees the app's types;
  - each static carries its place in the file, so the dialog orders every owner's settings as the file does, and its doc comment names its line in the schema.
- **The generator** is a library in `apertrust-base`, behind a `codegen` feature that only build-dependencies turn on.
  - It embeds the schema, so a change to the file reruns every owner's build script.
  - It reads the schema with toml_edit, the parser under `toml`, which keeps the file's order, where each key is and the comment above each header; the settings file's own reading and writing, through `toml`, are untouched by it.
  - It refuses at build time, naming the line: an unknown field, a field or value wrong for the kind, an unknown owner, kind or `applies`, a page the dialog does not have, a group without its page or a page without its group, a duplicate id, a default outside its range, and a missing default on a setting that is not `optional`. What rustc refuses in the generated code, it leaves to rustc: a choice's default none of its type's ids, a type that is no `Choice`, two ids with one name in Rust.
- **The registry** is the statics generated from the schema, taken together: what code, the store, `--conf` and the dialog read, as the actions registry is.
- **Other readers:** docs, the web client, scripts and MCP read the schema file itself.
- **Tests:** every id unique; every default within its range; every choice's type matching its default (the build's check); every message of a setting on a page, or of a tool setting, present in en-GB, uk and pl; and a version-1 `settings.toml` of every key read and written back byte for byte.

**Why a schema file rather than Rust declarations:** one file lists every setting of every owner, in the dialog's order, where Rust declarations would sit in three crates; a change to a setting is a change to data, reviewed as such; and docs, scripts, the web client and MCP read it without compiling Rust. The generated code is small, regenerated on every build and points back to the schema, and it makes typed statics, not widgets, so the dialog is still drawn from data at run time.

**darktable's idea, adapted.** darktable declares its 565 settings in `data/darktableconfig.xml.in`, 146 of them on the preferences dialog's seven tabs; XSLT generates `conf_gen.h` (defaults, types, ranges) and `preferences_gen.h` (the dialog). `conf.c` clamps a value to its range as it reads it, and writes the whole of `darktablerc` on exit.

- **Kept:** one data file declaring type, default and range, its order the dialog's; code generated from it at build time; values sanitised as read; the dialog from the declarations, a preference off its default marked and reset; restart notices; `--conf KEY=VALUE` for one run, never stored; `--configdir`.
- **Left:** the generated dialog, which GTK needs, since egui draws it from the statics at run time; options listed in the file, since a choice names its type; the whole-file write, which lets two darktables, or darktable and darktable-cli, overwrite each other's settings; `darktablerc-common`; and darktable's workspaces, a library with its own `darktablerc-<label>` chosen at start, which a user data folder of its own covers.

**The file's rules.**

- A missing key is its default. A value that cannot be read gets its default with a warning, and stays in the file until the user changes that setting; a number out of range is read as the nearer end. Unknown keys stay.
- A damaged file is set aside as `.bak`, and a newer version's is never overwritten, nor a read-only one.
- The file is written on a thread of its own a second after the last change and on exit, merging only the ids changed since the last write into the file as it is.
- Adding a setting never changes the version. A key that changes meaning or moves does, migrated on the TOML table before it is read.

**Engine settings** live under `[engine]`:

- `engine.adapter`: the first adapter whose name holds the text. Empty, the default, takes wgpu's high-performance choice. It applies after a restart, since the window's device is the engine's (decision 17). An adapter the setting names that is not there is warned of and wgpu's choice taken, as decision 7 makes a software adapter a fallback and not an error, so a settings file moved to another machine, or a GPU removed, never stops Apertrust. The app's window takes the adapter through eframe's adapter selector, the engine's `gpu::choose_adapter`, in the order wgpu's `request_adapter` sorts adapters where the setting names none.
- `engine.memory_budget`: 2048 MiB (decision 12), from 256 to 65536 MiB in steps of 256, kept whole. It applies after a restart. The CLI hands it to `export` and `preview`, and the app to its session, which takes it as it starts (`Session::start`) and plans every photo within it, so the Settings dialog's change applies at the next start, as its row says; at the next photo once the session takes a command for it.
- Planned: `engine.tone_mapper` (decision 33).

Where edits are kept is not a setting: each library and each root's id file decide it, never the user data folder (decision 47; [Edits in another user data folder](#edits-in-another-user-data-folder)).

**Tool settings** live under `[tools.<module>]`, in the schema with `owner = "module:<id>"` once their tool comes; the module's `Row::ToolSetting` then names the static generated for it in place of its kind. Until then a `Row::ToolSetting` keeps its kind (decision 29), since nothing keeps its value.

**`state.toml`**, beside `settings.toml`, holds Open Recent's folders, which name photos' places and so stay out of settings.

**Overrides.** `--conf KEY=VALUE` in the app and the CLI sets a value for one run, checked against the registry and never written; the dialog draws that row disabled.

- **Precedence:** a flag of its own (`--adapter`, `--max-binding`), then `--conf`, then the file, then the default. Each value is logged under `-d settings` with where it came from (`--adapter`, `--conf`, file, default). An empty `--adapter` takes wgpu's choice over the file's.
- **An adapter typed for this run** that is not there, by `--adapter` or `--conf`, is an error: what was typed for this run is never ignored, as a user data folder given explicitly never falls back.
- **`--conf`'s form:** `KEY=VALUE` or `--conf=KEY=VALUE`, repeatable, the last for a key counting; in the app for its settings and the engine's, in the CLI for the engine's and the export settings, global there.
- **Checked before the user data folder is looked for,** so a key no setting the program reads has, or a value its kind cannot read, stops the start, naming it, where darktable ignores either.
- **An empty value,** `KEY=`, is the empty text for a text setting (`engine.adapter=` takes wgpu's choice) and refused for any other kind, where darktable drops it.
- **Read by the setting's kind** with the file's rules: a number out of range at the nearer end, with a warning; a choice by its id; a text and a choice as typed, `~` included; anything else as TOML writes it, a list as `[1280, 800]`.
- **Held over the file** by `apertrust_base::settings::Overrides` and `Store::with_overrides`, skipped by the merge-write and reported by `Store::overridden`.
- **A set or a reset of an overridden setting is refused,** following darktable, whose `_conf_set_if_not_overridden` leaves the override's value for the run: the run keeps `--conf`'s value and the file keeps its own.
- Developer switches (`--max-binding`, `-d`) stay flags and are never settings. Environment variables only choose folders, besides the test ones and wgpu's own.

## Where it lives

**`apertrust-base`** (`crates/base`) holds only what every program sets up as it starts: logging and the banner, the version, folders and settings, the safe write and jobs. Nothing about images, colour, the GPU or the UI goes into it, and it depends on no egui, no wgpu and no engine. For settings it holds:

- the schema's generator, which embeds the schema from the top-level `data/` (decision 45);
- the `Setting` type, its kinds and the `Choice` trait;
- the store, `settings::Store`: `get` and `set` typed by the statics, owned for text, lists and tables; the file's rules above; the merge-write on its thread, a table's entries merged one by one; the migrations' hook, `MIGRATIONS`, none yet; `--conf`'s `Overrides`; and the `Storage` trait, `FileStorage` alone for now, which the browser build swaps (decision 35). A set that leaves the value as it is changes nothing. A computed default is no value: it follows what it is computed from, so a set keeps what it is given, and `reset` follows it again. `Store::read_only` reads by the same rules and borrows its storage, so it cannot write, set a damaged file aside or remove what other writes left: the CLI's;
- `Folders`, behind a `folders` feature that only the binaries turn on: the user data folder and how it was chosen. The cache folder comes with the thumbnails.

`apertrust-ui` turns on only base's settings part, for `Choice`, which its theme (`Builtin`), its language (`Locale`), the scopes' choices and the grid panel's (`GridPosition`, `SortKey`, `ThumbnailSize`) implement.

**The log is base's `log` module,** behind a `log` feature (decision 44). The golden harness, which reads no settings, turns on the log alone. Decision 44's version is computed in each binary's own build, by base's build-time helper behind a `build` feature (`apertrust_base::build::version()` in the binary's `build.rs`, read by `apertrust_base::program!()`), never in base's, so that a commit rebuilds the binaries and no library.

**Why logging and settings share a crate:** every program sets both up as it starts; both read the command line (`-d`, `--user-data-dir`, `--conf`); and they share the `-d settings` domain and the build version.

**Why "base":** it names a layer, as Chromium's `//base` (logging, the command line, files, the version) does. Not "common", which invites a grab bag, as darktable's `src/common/` became; not "core", confusable with the engine.

**Why the generator is in a crate:** a build script cannot use its own package's code, so a generator that the build scripts of the app, the engine and the modules share is a crate of its own; its refusals need tests, which a build script cannot hold; and `apertrust-ui` implements `Choice` for the theme and the language, so the trait sits in a crate below both the design system and the app.

**Few crates** (decision 65): new code goes into a module of an existing crate, base's behind a feature (`jobs`, `fs`) or the library's (`sidecar`, `listing`, `index`). A crate of its own needs a reason: `unsafe` FFI, a dependency direction that would otherwise break, or a decided boundary such as an operation's (decision 20). So the safe write is `apertrust_base::fs` and the sidecar's text layer `apertrust_library::sidecar`, and `apertrust-fs` is made only if bulk listing needs FFI that `rustix` and the `windows` crate do not cover safely.

**Crates that stay apart,** each for its reason: `apertrust-display` (FFI, allowed `unsafe`); `apertrust-raw` (the rawler dependency); `apertrust-color` (the kernels' Slang import path); `apertrust-plugin`, `apertrust-operations` and `apertrust-modules` (layer boundaries); `apertrust-ui` (egui).

**Each owner's crate generates its own settings:**

- the app's, in its `build.rs`;
- the engine's, in `apertrust-engine`'s, as `apertrust_engine::settings`, whose `Options::read` builds `GpuOptions` and the budget from a store;
- tools', in `apertrust-modules`', with the first tool;
- the export settings, the Export dialog's, in `apertrust-export`'s, a third owner, `export` (decision 56): `apertrust_export::settings::registry`, written by the app, which keeps the dialog's last settings there as darktable keeps its conf, and read by the CLI's store with the engine's, its `export` taking the format's options from it, as darktable-cli reads darktablerc.

**The engine never opens a folder or reads a file of settings.** The binaries read the store and hand it plain options.

- **The CLI** reads its user data folder's engine, export and tool settings and never writes settings. It finds the folder with `Folders::find_to_read`, which creates and probes nothing, and its store is `Store::read_only`. A test runs it on files of every kind, damaged and newer included, and finds each byte for byte as it was, nothing set aside and no temporary file removed. Only the commands that run the engine (`gpu`, `preview`, `export`) open the store.
- **The app** opens one store on its registry and the engine's and the export's together, so their keys share its file and are never unknown to it.
- **The golden harness** reads none, so no result depends on a person's folder. It turns on base's log alone, opens no store, and takes `GpuOptions::default()` and `pipeline::BUDGET`, which a test of the engine holds equal to these settings' defaults.
- **Remotely** (Phase 7), engine settings are the server's folder's, and a client's own settings are the client's.

## User data folders

**The term** is the **user data folder**, as Chromium, VS Code and Electron call the same idea. It holds the person's settings and personal resources, not only configuration, so "config folder" would not fit; darktable's configdir has that mismatch, since it holds `library.db` too. The name says what the folder holds; its place follows each platform's custom.

**How the folder is chosen,** in this order:

1. `--user-data-dir DIR` or `--user-data-dir=DIR`, with `--config-dir` and darktable's `--configdir` accepted as aliases, so darktable users' habit works; the last given counts;
2. then `APERTRUST_USER_DATA_DIR`, empty counting as unset, as XDG's variables do;
3. then the platform's folder: `~/Library/Application Support/Apertrust` on macOS, `%APPDATA%\Apertrust\config` on Windows, `$XDG_CONFIG_HOME/apertrust`, by default `~/.config/apertrust`, on Linux.

- **Paths:** a relative path is taken from the current folder. A leading `~` in the flag's folder or the variable is the home folder, as darktable has it, `~` alone or `~/…` and never `~user`, since the shell leaves it after `=`.
- **The app** (`Folders::find`) creates a missing folder and probes that it can write there. A folder given explicitly that cannot be created or written stops the start with an error naming it, and never falls back to the platform's, which is what the person asked to keep away from. A flag without its folder stops the start too. The platform's folder, when it cannot be created, is warned of and used all the same.
- **The CLI** (`Folders::find_to_read`) only reads, so it creates and probes nothing, and a read-only folder serves it. A folder given explicitly that is missing or cannot be read stops it, naming it; a missing platform folder stays missing, its settings the defaults. It takes the flag through clap in both forms, before or after its command, with its aliases shown in `--help`, and prints a folder error once, its cause in its line.
- **`-d settings`** logs the folder and how it was chosen.

**One whole folder on Linux,** as darktable's `~/.config/darktable`, VS Code's and Chromium's are, rather than XDG's split by kind (settings in `$XDG_CONFIG_HOME`, the rest in `$XDG_DATA_HOME`, Open Recent and reports in `$XDG_STATE_HOME`): one place to find and back up, laid out as a chosen `--user-data-dir` is. Windows keeps its `config` subfolder.

**What it holds:** the person's settings and personal resources.

- `settings.toml` and `state.toml`;
- `keywords.toml`, folder mode's keyword properties and Recent, of the same form as a library's (planned, decision 73);
- `themes/` (decision 36);
- later scripts and similar personal resources, `styles/`, `presets/` and `reports/` (crash reports and the log file; planned with the crash report, which replaces the home folder with `~` and photo and folder names with placeholders, keeping file extensions and camera models, while the local log file keeps full paths, decision 44). Export presets come first: `presets/export.toml`, a table per preset keyed by its name and holding the `export.*` settings' keys, read with the settings' rules, written by the app through the store's merge-write and read by `apertrust-cli export --preset NAME` (decision 56, planned).

No other platform folder is used, XDG's data and state folders among them, so one folder holds a person's whole set-up; the cache is apart, below.

**The library does not live there.** A library is a folder of its own with `Name.aplibrary` at its top, wherever the person puts it, next to the images ([library.md](library.md#a-library)). Of the library, the user data folder holds only project templates (decision 46); each library's per-machine cache and root map are in the cache folder (decision 55). Edits are where each root's id file says, never decided by the user data folder (decision 47).

**The cache folder** holds what Apertrust can compute again: thumbnails, compiled pipelines, later embeddings and depth maps.

- By default it is the platform's cache folder: `~/Library/Caches/Apertrust`, `%LOCALAPPDATA%\Apertrust\cache`, `$XDG_CACHE_HOME/apertrust` (`~/.cache/apertrust`).
- With a user data folder given by flag or variable, it is `<user data>/cache`, since a development build changes kernels without bumping the thumbnails' render version and could otherwise show its thumbnails in the person's Apertrust.
- `--cache-dir` and `APERTRUST_CACHE_DIR` will put it elsewhere, as darktable's `--cachedir`, added with the first cache.

**Two instances on one folder** are allowed.

- Settings and `state.toml` merge per key.
- Cache entries are keyed by content and written atomically.
- Two instances, or the app and `apertrust-cli library`, share a library through its files, with no lock: each writes whole files only when their bytes change, reads a file again before writing it if it changed, and sees the other's changes as any outside change (decisions 52 and 54). Each keeps its own cache.

**Development builds** are Apertrust Dev, which installs and runs beside Apertrust ([packaging.md](packaging.md#apertrust-dev)), as nightly channels of other apps do.

- **Which builds are Dev:** the nightly packages are always Dev; the nightly job says so explicitly (`tools/package.sh --dev`, which builds with `APERTRUST_DEV=1`). Every other build, from source or `cargo run`, stays Apertrust while no release tag exists, and becomes Dev once one does: a build not on a release tag, when the history has one (`X.Y.Z+N~gHASH` or `-dirty`, decision 44). The rule is one condition where the version is computed (`crates/base/src/build/version.rs`); a packager's `APERTRUST_VERSION` is a release.
- **Its folder:** the platform's folder is Apertrust Dev's, `~/Library/Application Support/Apertrust Dev` on macOS, `%APPDATA%\Apertrust Dev\config` on Windows and `$XDG_CONFIG_HOME/apertrust-dev` on Linux, for the app and the CLI alike (`Folders::find(given, dev)`). A folder given by flag or variable is used as given. Its cache will be its own, with the first cache.
- **Its name:** the window's title, the banner (`Apertrust Dev 0.1.0+12~gabc1234 [...]`, `Apertrust Dev CLI ...`) and the app's id, `org.apertrust.Apertrust.Dev`.
- It separates a development build's settings only: its edits are the person's sidecars beside the photos, and a development build that understands an edit can still change it (below).

## Edits in another user data folder

- **A user data folder never decides where edits go** (decision 47). Every user data folder reads and writes a photo's edits where its root's id file says, beside the photo by default (decision 48), or in a library's own folder ([library.md](library.md#where-edits-live)). `--user-data-dir` separates settings only, and so does Apertrust Dev; `--help` and the docs say so.
- **The risk that remains,** stated plainly: a development build reads and writes the person's sidecars. The rule below stops it overwriting an edit that holds what it cannot keep, but a development build that understands an edit can still change it. Once it writes what the installed Apertrust lacks, a control the installed one does not have, say, the installed one opens that photo for viewing only.

**No build overwrites an edit a newer Apertrust wrote,** in any folder ([engine.md](engine.md), Edits, What a save may drop). Newer is told by what the edit holds, never by comparing builds' versions, which development builds on different branches do not order. It is anything in the edit that this build would drop on saving:

- a newer file version or module version;
- a module this build neither knows nor has retired;
- at a module's own version: a control it lacks, or a value it cannot read or keep (an option it lacks, an option or control not ported, a number out of range);
- an operation it lacks, a schema version above its own, or at its own schema a parameter it lacks;
- an entry repeated, or any element or attribute it does not write.

Then:

- the photo opens for viewing only, rendered from what this build reads, its controls disabled, saying which version wrote it;
- a file version it cannot read at all refuses to open, naming the version;
- what it knows an older build wrote is dropped with a warning, and the photo stays editable: a module Apertrust has retired, as the built Tone curve's `tone_curve` once it becomes RGB curve (decision 2), what an older module or schema version held that this one lacks, a parameter a module now owns, an optional operation's parameters, and version 0;
- every edit records the build that wrote it, decision 44's version rather than `Cargo.toml`'s.

**Versions rise for removals and changes, never for additions.** A module's version, an operation's schema version and the file's version rise when what an older build wrote would read differently or not at all: a control, option or parameter removed or renamed, a range narrowed, a default changed, a meaning or binding changed, or a layout an older build would misread.

- The migration keeps the look. While no released edit can hold the value, it may instead drop it with a warning, as Exposure's version 2 will drop the Contrast and Pivot it held at version 1.
- An addition whose default keeps old edits' look raises nothing: a build without it finds it in the edit and opens the photo for viewing only.
- A raise for every addition, as the only guard, would not do: two branches' versions of the same number differ, so one branch's addition would be lost in the other's.

**Held by tests:**

- a committed record of each module's stored controls and of the operation parameters an edit can store (those no module owns, of operations an unedited photo runs). It fails a removal or change until the version rises, an addition until the record is blessed, and a module or operation recorded but neither built in nor retired;
- a committed version-1 sidecar that reads with no warning and writes back byte for byte.

## The Settings dialog

Drawn on nine pages, as darktable's preferences: General, Import, Library, Editing, Processing, Storage, Shortcuts, Presets and AI ([gui.md](gui.md#the-settings-dialog) holds its form, its rows and its window). It is a native window of its own (decision 41), on the Mac a modeless one rather than a sheet.

- **Rows from the registry:** each setting the schema gives a `page` and a `group` is a row on that page, under its group's title, in the schema's order, drawn by its kind. A row off its default has darktable's dot and is reset by a double-click on its label or its right-click menu's Reset; a row `--conf` sets is drawn disabled, saying so; "Applies after a restart" is said where the schema says so. Every change applies at once.
- **A list of a choice** (`kind = "list"`, `of = { kind = "choice", type = "…" }`) is a row of its own kind: a checkbox per option of the type, stacked in the control column in the type's order, the row as tall as they are, with one dot and one Reset for the whole list; a change sets the list to the options ticked, in the type's order. It is generic, for any such list on a page; the options' words are `setting-<id>-option-<option>`, as a choice's.
- **A row's help** is its words' `setting-<id>-help`, where it has one, shown as its label's tooltip.
- **Diagnostics,** General's third group: `log.keep` (a boolean, off by default, applied at once), whether the app keeps this session's log in memory for its log window, and `log.domains` (a list of `apertrust_base::log::Domain`, the `-d` domains but `golden`, then `verbose`; all but `memory` and `verbose` by default, applied at once), what it keeps besides warnings and errors ([gui.md](gui.md#the-log-window)). The list is kept while the log is off, and used once it is on.
- **Hand-built pages** for what rows cannot hold: Shortcuts, the keymap editor; Storage, the user data and cache folders, each with how it was chosen, read-only, since a folder is chosen before the app starts, and the thumbnail cache; Presets; AI's models. Where edits are kept is no setting: each library's file and each root's id file say (decision 47).
- **A page that holds nothing yet** is listed dimmed and cannot be chosen. Built: General, Library, Editing and Processing; Import, Storage, Shortcuts, Presets and AI are dimmed.
- What the app remembers on its own (the window's placement, the sections open, the scopes' view, the tab shown, the grid panel's position, size, sort, thumbnails and whether they show their captions, `grid_panel.labels`, the grid bar's Labels, the page Settings last showed, `settings_window.page`, and the log window's place, its size and the folder its Save to file… last wrote to, `log_window.*`) is never shown, and tool settings are drawn in their module.
- **Square thumbnails,** `grid_panel.square_thumbnails` (a boolean, off by default, applied at once; [gui.md](gui.md#the-cell)), is the Library page's row, under Thumbnails; `--conf grid_panel.square_thumbnails=true` turns it on for a run.
- darktable's seven tabs hold lighttable and darkroom state that one view does not have, so they are not mirrored row for row: Apertrust mirrors darktable's modules, not its shell.
