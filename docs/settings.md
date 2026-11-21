# Settings and folders

How Apertrust's settings are declared, generated, stored and read, which folders it keeps them in, and how an edit is kept safe from another user data folder or another build (decision 43). What the desktop app keeps in its settings file and how it draws the Settings dialog are [gui.md](gui.md#settings-and-workspace)'s; the crates' state is in [crates.md](crates.md).

**Status.** Built: the schema with the boolean, number, choice, text, list and table kinds, a number's unit and factor, each setting's page and group, a row's `requires` and its `platforms`, its generator and every owner's registry; the store with the file's rules and the merge-write; `--conf`; the user data folder with its flags and variable; the cache folder with its flag and variable, and the cache's three settings (decision 76); development builds' own folders, Apertrust Nightly's; edits a newer Apertrust wrote opened for viewing only; the Settings dialog's pages of rows, and Storage's hand-built part. The app opens the cache and its session keeps it within `cache.limit`, a change applying at once (decision 76). Planned: whole numbers and folders in the schema, each with the first setting that needs one; the Settings dialog's other hand-built pages, Shortcuts first.

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

- **its id,** the table's header: the dotted path of its key in `settings.toml`, stable within a file version, a key that moves needing a versioned migration ([below](#the-settings-schema), The migrations), its parts snake\_case (`scale`, `tools_panel.width`, `engine.memory_budget`, `tools.exposure.lightness`);
- **`owner`:**
  - `app`: read by the desktop app alone;
  - `engine`: read by every process that runs the engine, the app, the CLI and later the server;
  - `export`: the Export tool's settings (decisions 56 and 90), kept by the app and read by the CLI's `export`;
  - `import`, planned with step 20a: Import's settings for the copy itself, the base folder, the default folder, subfolder and file name patterns, the job code (Import name in the sheet), the date override (scripts' alone) and the rest of what `PreviewImport` and `Import` carry (decision 85), kept by the app and read by the CLI's `import`; the Import sheet's own places, folder, the folder last imported into, the Folder and Subfolder menus' recents, column width, Keep open and Zoom are `app`'s, under `import.` too (decision 86);
  - `tool:<id>`: a tool's setting (decision 29), kept by the client and sent with the commands that use it;
- **`kind`** and its fields:
  - boolean;
  - number: `min`, `max`, `step`, `decimals` (how many it is kept and shown with), `unit`, the id of the words it is shown with (`unit-<unit>`, as `mib`), and `factor`, what it is multiplied by where it is shown, as darktable's slider factor (`scale`, kept as 1.1, shows as 110 %); or, in place of `min`, `above`, what it must be more than, as a size's parts must be more than 0;
  - whole number, choice (`type`), text, folder;
  - list: `of`, its items' kind, and `length` when it is set;
  - table keyed by another registry's ids: `by` (`actions`, `sections`, `tools` or `groups`, and `tabs`, planned with the Tools panel's layout, decision 83) and `of`, its values' kind, as `[edit.groups]`'s `by = "tools"` and `of = { kind = "table", by = "groups", of = "boolean" }`. An item's or a value's kind is a kind's name or an inline table of `kind` and that kind's fields;
  - record, planned with floating panels (decision 83): a table of named fields, each of a kind, as a floating panel's `corner`, `x`, `y` and `width` under `tools_panel.floating`;
- **`default`:** a value, or `"computed"` for one computed in Rust at start (the language follows the system's). A setting with no value until its owner keeps one, as the window's position, has none and says **`optional = true`**, which makes its static's value an `Option`. Any other setting without a default is refused;
- **`page`**, one of the Settings dialog's nine, `general`, `import`, `library`, `editing`, `processing`, `storage`, `shortcuts`, `presets` or `ai` ([The Settings dialog](#the-settings-dialog)), and **`group`** on it, snake\_case, each with the other; or neither, for what the app keeps by itself (the window's placement, the sections open, the scopes' view, the Browser's layout, the page Settings last showed). A page's groups and rows are in the schema's order;
- **`requires`,** on a setting with a page: the id of another boolean setting its row needs on, the row drawn disabled while that is off, as `cache.limit`'s while `cache.limited` is; the generator refuses one that names no other boolean setting, or one on no page, whose row is never drawn to turn on;
- **`applies`:** `at-once`, `next-photo` or `after-restart`;
- **`platforms`,** on a setting with a page, as an action's: a list of `windows`, `macos` and `linux`, each once, the platforms the dialog draws its row on, every one without it, as `engine.backend`'s `["windows"]`. Only the row is left out: the value is read, kept and taken by `--conf` on every platform alike, so a settings file carried between computers keeps it. The generator refuses any other list, and one on a setting with no page;
- **a `#` comment** above it: a developer's note, as darktable's longdescription, which its static's doc comment carries.

**Built so far:** the boolean, number, choice, text, list and table kinds; `default`, `optional`, `owner`, `applies`, `page`, `group`, `requires` and `platforms`, for the graphics API and the menu button; a number's `above`, for the window's size and scale factor, and its `unit` and `factor`, for the interface's size and the memory budget; and each owner's `ALL`, every setting it owns, the registry its store reads. Whole numbers and folders come with the first setting that needs one.

**Words** stay in Fluent, by ids that follow from the id, as a tool's do: `setting-<id>-label`, `-full`, `-promise` and `-option-<option>`, the id's dots and underscores as hyphens, with `settings-page-<page>` and `settings-group-<group>`. A tool's setting keeps its tool's, `tool-<tool>-setting-<setting>-…` (decision 29). The schema holds no English.

**How it becomes code:**

- **A choice names its Rust type** instead of listing its options, so no type is declared twice. The options come from the type through `Choice`, a small trait in `apertrust-base`: each value's stable id and every value in the dialog's order, as associated constants. The generated code checks the default against them in a constant, so a default no value has fails the build. A computed default calls a function the owner's crate defines under the setting's name, so a missing one fails the build too.
- **Each owner's crate generates its own settings** in its `build.rs`: the app's for `owner = "app"`, the engine's for `engine`, `apertrust-export`'s for `export`, `apertrust-library`'s for `import` (planned, decision 85), `apertrust-tools`'s for `tool:<id>`. They are typed statics written into `OUT_DIR`, not committed:
  - `settings.get(&SCALE)` and `settings.set(&SCALE, value)` are checked by the compiler;
  - the engine's generated code never sees the app's types;
  - each static carries its place in the file, so the dialog orders every owner's settings as the file does, and its doc comment names its line in the schema.
- **The generator** is a library in `apertrust-base`, behind a `codegen` feature that only build-dependencies turn on.
  - It embeds the schema, so a change to the file reruns every owner's build script.
  - It reads the schema with toml_edit, the parser under `toml`, which keeps the file's order, where each key is and the comment above each header; the settings file's own reading and writing, through `toml`, are untouched by it.
  - It refuses at build time, naming the line: an unknown field, a field or value wrong for the kind, an unknown owner, kind or `applies`, a page the dialog does not have, a group without its page or a page without its group, a duplicate id, a default outside its range, and a missing default on a setting that is not `optional`. What rustc refuses in the generated code, it leaves to rustc: a choice's default none of its type's ids, a type that is no `Choice`, two ids with one name in Rust.
- **The registry** is the statics generated from the schema, taken together: what code, the store, `--conf` and the dialog read, as the actions registry is.
- **Other readers:** docs, the web client, scripts and MCP read the schema file itself.
- **Tests:** every id unique; every default within its range; every choice's type matching its default (the build's check); every message of a setting on a page, or of a tool's setting, present in en-GB, uk and pl; a version-2 `settings.toml` of every key read and written back byte for byte; a version-1 file holding every renamed key, action id and Browse entry read with every value kept under its new name and written as version 2 with the new keys alone; one holding an old key and its new one keeping the new one's value, Browse's groups joining Collections'; and a file with no version read as version 1, its old keys and key bindings moved and kept through a write.

**Why a schema file rather than Rust declarations:** one file lists every setting of every owner, in the dialog's order, where Rust declarations would sit in three crates; a change to a setting is a change to data, reviewed as such; and docs, scripts, the web client and MCP read it without compiling Rust. The generated code is small, regenerated on every build and points back to the schema, and it makes typed statics, not widgets, so the dialog is still drawn from data at run time.

**darktable's idea, adapted.** darktable declares its 565 settings in `data/darktableconfig.xml.in`, 146 of them on the preferences dialog's seven tabs; XSLT generates `conf_gen.h` (defaults, types, ranges) and `preferences_gen.h` (the dialog). `conf.c` clamps a value to its range as it reads it, and writes the whole of `darktablerc` on exit.

- **Kept:** one data file declaring type, default and range, its order the dialog's; code generated from it at build time; values sanitised as read; the dialog from the declarations, a preference off its default marked and reset; restart notices; `--conf KEY=VALUE` for one run, never stored; `--configdir`.
- **Left:** the generated dialog, which GTK needs, since egui draws it from the statics at run time; options listed in the file, since a choice names its type; the whole-file write, which lets two darktables, or darktable and darktable-cli, overwrite each other's settings; `darktablerc-common`; and darktable's workspaces, a library with its own `darktablerc-<label>` chosen at start, which a user data folder of its own covers.

**The file's rules.**

- A missing key is its default. A value that cannot be read gets its default with a warning, and stays in the file until the user changes that setting; a number out of range is read as the nearer end. Unknown keys stay.
- A damaged file is set aside as `.bak`, and a newer version's is never overwritten, nor a read-only one.
- The file is written on a thread of its own a second after the last change and on exit, merging only the ids changed since the last write into the file as it is.
- Adding a setting never changes the version. A key that changes meaning or moves does, migrated on the TOML table before it is read.

**The migrations,** `MIGRATIONS` in `crates/base/src/settings/migrations.rs`, each from one version to the next, run on the file's table before it is read and before each write merges into it. The file is version 2. The one migration so far, from version 1, came with the window's parts named by their role (decision 95):

- **It moves** each key of `[side_panel]`, `[central_panel]` and `[grid_panel]` to `[tools_panel]`, `[viewer]` and `[browser]`; the action ids `show-browse` and `grid-position-left`, `-right` and `-bottom` in `[keys]` to `show-collections` and `browser-position-left`, `-right` and `-bottom`; and Browse's entries in `edit.sections` and `edit.groups`, `browse`, to `collections`.
- **Where both are present,** as only a hand edit makes them in a version-1 file, the new key wins and the old is dropped, with a line under `-d settings` naming it. Browse's groups join Collections' one by one, each of Collections' winning.
- **Every key that can move keeps its value:** a version-1 file is read migrated at every start, and written as version 2, holding the moved keys under their new names alone, at its next write. A file with no version, or one that cannot be read, is read as version 1, with a warning, so its old keys move too.
- **An old key whose new place holds something other than a table,** again only a hand edit's doing, stays on disk where it is, without effect: the file is still written as version 2, so the key is kept but never moved or read again, and `-d settings` names it.
- **`--conf` takes the new keys only;** an old one stops the start, as any key no setting has does. An older build reads a version-2 file for what it knows and never overwrites it, so it shows these settings at their defaults.

**Engine settings** live under `[engine]`:

- `engine.adapter`: the adapter whose name is the text, ignoring case, else, for a name typed by hand, the first whose name holds it (`gpu::matching`); `cpu`, reserved, the chosen API's software adapter (WARP, lavapipe) whatever its name, so a driver or Mesa update never breaks the choice (decision 82). Empty, the default, takes wgpu's high-performance choice. `--adapter cpu` and `--conf engine.adapter=cpu` take the software adapter for one run, an error where the API has none. It applies after a restart, since the window's device is the engine's (decision 17). An adapter the setting names that is not there is warned of and wgpu's choice taken, as decision 7 makes a software adapter a fallback and not an error, so a settings file moved to another machine, or a GPU removed, never stops Apertrust. The app's window takes the adapter through eframe's adapter selector, the engine's `gpu::choose_adapter`, in the order wgpu's `request_adapter` sorts adapters where the setting names none.
  - **Its row** is a list, though the schema keeps a text (decision 82; [gui.md](gui.md#the-settings-dialog)): Automatic, alone, the In use line saying what it runs, then each hardware adapter the chosen API finds, each name once, then **CPU (no graphics card), slow**, stored as `cpu`; listed by a helper process, the app's own executable with `--list-adapters`, so a driver that crashes as it loads ends the helper and not the app; a card the chosen API lacks, or one no longer there, said so on the row in the warning colour; and an "In use" line under the group's rows naming the adapter and API running now, with a warning where the software adapter runs.
  - **Restart now:** while a setting that applies after a restart has a value other than the one this run started with, the line "Your choice applies when Apertrust restarts" under In use has a Restart now button, which quits as Quit does and starts the app again with the same flags and what is open ([gui.md](gui.md#the-settings-dialog)).
  - **Set aside for a run:** `--safe-mode` opens Automatic; else a start after one that did not finish, which left `starting.toml` in the user data folder unlocked (one another start holds locked is a start under way), opens Automatic, or the CPU where Automatic was what failed and the API has one, leaving out the API that did not start where Automatic loads another (on Windows, Direct3D 12 for Vulkan and Vulkan for Direct3D 12, a start on the API left that does not finish either giving the CPU; the CPU on Direct3D 12 alone); either keeps both settings in the file and the rows as they are (`engine::settings::Options::instead`, logged under `-d settings` as from `--safe-mode` or "a start that did not finish"). The window's notice offers Keep Automatic (or Keep Automatic on the API left, or Keep CPU), which sets them to what runs, the API included, and Try again, which restarts with them; while `--conf` sets either, Keep changes nothing, and the notice stays, saying why.
- `engine.backend` (decision 82): the graphics API the engine's wgpu instance loads, and the app's window with it, built from the same `GpuOptions`: `auto`, the default, wgpu's primary backends (Direct3D 12 and Vulkan on Windows, Vulkan on Linux, Metal on macOS, never GL), or `dx12` or `vulkan` alone (`gpu::Backend`). It applies after a restart, and its row, Graphics API, is drawn on Windows alone (`platforms = ["windows"]`), the only platform with a choice. There is no `metal`: on macOS Automatic is Metal. wgpu's `WGPU_BACKEND`, where set, comes before it.
  - **A settings file never stops Apertrust over it.** A value is read on every platform. The file's, where this build lacks its API (Direct3D 12 off Windows, Vulkan on macOS) or where that API lists no adapter on this machine (Vulkan without a Vulkan driver, as in a virtual machine, over Remote Desktop or on an old GPU), is warned of under `-d gpu` and Automatic taken, as `settings::Options::read` reads it (`gpu::finds_adapter`), before the app's window or any engine device is made, so the window and the engine load the same backends.
  - **Typed for the run,** by `--backend` or `--conf`, it is kept: one this build lacks is an error as the engine starts (`gpu::check_backend`; the app checks it before its window opens), and one that finds no adapter fails as the device opens ("no usable GPU adapter").
- `engine.memory_budget`: 2048 MiB (decision 12), from 256 to 65536 MiB in steps of 256, kept whole. It applies after a restart. The CLI hands it to `export` and `preview`, and the app to its session, which takes it as it starts (`Session::start`) and plans every photo within it, so the Settings dialog's change applies at the next start, as its row says; at the next photo once the session takes a command for it.
- Planned: `engine.tone_mapper` (decision 33).

Where edits are kept is a folder's own setting, Keep Edits on This Computer, kept in its folder record in the user data folder, with `library.keep_edits_here` for every folder not yet edited (planned, decisions 47 and 101; [Edits in another user data folder](#edits-in-another-user-data-folder)).

**A tool's settings** live under `[tools.<module>]`, in the schema with `owner = "tool:<id>"` once what uses them is built, generated by `apertrust-tools`' `build.rs` into `apertrust_tools::settings::registry`. The tool's `Row::Setting` keeps its kind, darktable's range, options and default (decision 29), and finds its setting by its id, `tools.<module>.<setting>`; a test holds the two to agree. One with no setting yet is drawn disabled at its default, since nothing keeps its value. Built: Exposure's Picker target, Lightness and reference, `tools.exposure.target`, `tools.exposure.lightness`, and `tools.exposure.reference_lightness` and `reference_photo`, optional, none until a reference is set and removed again as it is cleared (decision 88; an old `tools.exposure.area_mode` stays as an unknown key) ([pickers.md](pickers.md#a-tools-settings)). The app's store reads them with its own; the CLI runs no picker and reads none.

**`state.toml`**, beside `settings.toml`, holds the recents, Open Recent's list, and what was open last, which name photos' places and so stay out of settings (the app's `recents`, `crates/app/src/recents.rs`; the CLI reads and writes nothing of it):

```toml
version = 1

[last]
folder = "/Users/me/Pictures/Wedding/Day 1"
kind = "folder"
path = "/Users/me/Pictures/Wedding"
photo = "IMG_2041.ARW"

[[recent]]
kind = "folder"
opened = 1794308254
path = "/Users/me/Pictures/Wedding"
```

- **The list:** newest first by `opened`, in whole seconds since 1970, at most ten; an entry opened goes to the top, its `opened` one past the newest's where the clock was set back, so a full list never cuts it off. `kind` is `folder`; a Library, by its id, joins it in Phase 5 (decision 101).
- **Written** on a thread of the list's own, through the safe write, each time the list changes and on exit. Each write reads the file again and merges into it each change made since the last write (an entry opened, removed, re-pointed, or the list cleared), so a change made meanwhile outside the app is kept; the file's other keys stay. One instance writes it, the folder's (decision 89). A write that fails keeps its changes for the next write, and the one on exit; a file that may not be written is such a failure, with a warning.
- **What cannot be read:** an entry of a kind this build lacks, or a malformed one, stays in the file, in its place, unshown; a malformed one of a kind it knows is warned of. A damaged file, not TOML or not UTF-8, is set aside as `state.toml.bak` with a warning that quotes nothing of it: at the start the list starts empty, and a later write writes what the last write left, the list and the tabs, with its change. One a newer version wrote is read as empty and never written.
- **What was open last,** `[last]`, is what the next start reopens while `library.reopen_last` is on (on by default; Settings › General's Start group; [gui.md](gui.md#the-start-window)): `path`, a folder opened in folder mode, Collections' root, written as it opens; `folder`, the folder of its tree active as the app ended, where it was another, and `photo`, the file name of the photo active then in that folder, never one whose open failed, added as the app ends while that root is still Collections', and kept as they were while a reopen is under way. A photo opened alone, by any route (decision 46), never touches it, so one photo is never reopened. File › Close of that root takes it out; Clear Menu leaves it. A write merges it as it does the list, the newest change winning, and one this build cannot read is read as none and left in the file until replaced.
- **Paths** are written as text, a path that is not valid Unicode with its invalid bytes replaced.
- **Missing entries:** whether each entry's path is there is asked as the main window shows, and again as it gains the focus and as Collections comes into sight, each on a thread of its own, never the interface's, so a slow share never holds a frame; a path still being asked is not asked again. One found missing is marked, and unmarked once a later check finds it back; one not yet answered is not, and an answer to a question asked before the entry was opened again is dropped. Remove from the List and Locate…'s re-pointing are the list's own (`Recents::remove`, `Recents::repoint`), and so is Clear Menu (`Recents::clear`).
- **The tab last shown in each workspace** (decision 96): `[tab]`, keyed by workspace id (`default = "adjust"`), set only when the user chooses a tab, so one this build lacks stays until then, and merged per change as the list is; a value that is no text is read as none and left in the file until replaced. `edit.tab`, its old settings key, stays in the schema as `library.recent_folders` does: read only where `state.toml` holds no tab for Default, for its move there, the file's own value, never `--conf`'s, written at once only where the file still holds none for Default as it is merged, and never written by the app.
- **`library.recent_folders`,** the recent folders' old settings key, stays in the schema: read only where `state.toml` holds no list yet, for its move there, written at once, and never written by the app.
- **The old key stays in `settings.toml`** after the move, as an earlier build wrote it.

**Overrides.** `--conf KEY=VALUE` in the app and the CLI sets a value for one run, checked against the registry and never written; the dialog draws that row disabled.

- **Precedence:** a flag of its own (`--adapter`, `--backend`, `--max-binding`), then `--conf`, then the file, then the default, and for `engine.backend` wgpu's `WGPU_BACKEND` before them all. Each value is logged under `-d settings` with where it came from (`WGPU_BACKEND`, `--adapter`, `--backend`, `--conf`, file, default). An empty `--adapter` takes wgpu's choice over the file's.
- **An adapter typed for this run** that is not there, by `--adapter` or `--conf`, is an error: what was typed for this run is never ignored, as a user data folder given explicitly never falls back.
- **`--conf`'s form:** `KEY=VALUE` or `--conf=KEY=VALUE`, repeatable, the last for a key counting; in the app for its settings, the engine's, the export settings and the tools' settings (`tools.*`), in the CLI for the engine's and the export settings, global there.
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
- the store, `settings::Store`: `get` and `set` typed by the statics, owned for text, lists and tables; the file's rules above; the merge-write on its thread, a table's entries merged one by one; the migrations, `MIGRATIONS`, one so far, from version 1 to 2 (decision 95, above); `--conf`'s `Overrides`; and the `Storage` trait, `FileStorage` alone for now, which the web browser build swaps (decision 35). A set that leaves the value as it is changes nothing. A computed default is no value: it follows what it is computed from, so a set keeps what it is given, and `reset` follows it again. `Store::read_only` reads by the same rules and borrows its storage, so it cannot write, set a damaged file aside or remove what other writes left: the CLI's;
- `Folders`, behind a `folders` feature that only the binaries turn on: the user data folder and the cache folder, each with how it was chosen. The cache itself is in a `cache` feature (decision 76, below), which the engine turns on for `UnusedAfter` and `Limit`.

`apertrust-ui` turns on only base's settings part, for `Choice`, which its theme (`Builtin`), its language (`Locale`), the scopes' choices and the Browser's (`BrowserPosition`, `SortKey`, `ThumbnailSize`) implement.

**The log is base's `log` module,** behind a `log` feature (decision 44). The golden harness, which reads no settings, turns on the log alone. Decision 44's version is computed in each binary's own build, by base's build-time helper behind a `build` feature (`apertrust_base::build::version()` in the binary's `build.rs`, read by `apertrust_base::program!()`), never in base's, so that a commit rebuilds the binaries and no library.

**Why logging and settings share a crate:** every program sets both up as it starts; both read the command line (`-d`, `--user-data-dir`, `--conf`); and they share the `-d settings` domain and the build version.

**Why "base":** it names a layer, as Chromium's `//base` (logging, the command line, files, the version) does. Not "common", which invites a grab bag, as darktable's `src/common/` became; not "core", confusable with the engine.

**Why the generator is in a crate:** a build script cannot use its own package's code, so a generator that the build scripts of the app, the engine and `apertrust-tools` share is a crate of its own; its refusals need tests, which a build script cannot hold; and `apertrust-ui` implements `Choice` for the theme and the language, so the trait sits in a crate below both the design system and the app.

**Few crates** (decision 65): new code goes into a module of an existing crate, base's behind a feature (`jobs`, `fs`) or the library's (`sidecar`, `listing`, `index`). A crate of its own needs a reason: `unsafe` FFI, a dependency direction that would otherwise break, or a decided boundary such as an operation's (decision 20). So the safe write is `apertrust_base::fs` and the sidecar's text layer `apertrust_library::sidecar`, and `apertrust-fs` is made only if bulk listing needs FFI that `rustix` and the `windows` crate do not cover safely.

**Crates that stay apart,** each for its reason: `apertrust-display` (FFI, allowed `unsafe`); `apertrust-raw` (the rawler dependency); `apertrust-color` (the kernels' Slang import path); `apertrust-plugin`, `apertrust-operations` and `apertrust-tools` (layer boundaries); `apertrust-ui` (egui).

**Each owner's crate generates its own settings:**

- the app's, in its `build.rs`;
- the engine's, in `apertrust-engine`'s, as `apertrust_engine::settings`, whose `Options::read` builds `GpuOptions` and the budget from a store;
- the tools', in `apertrust-tools`', as `apertrust_tools::settings::registry`, Exposure's first;
- the export settings, the Export tool's, in `apertrust-export`'s, a third owner, `export` (decision 56): `apertrust_export::settings::registry`, written by the app, which keeps the tool's settings there as darktable keeps its conf, and read by the CLI's store with the engine's, its `export` taking the format's options from it, as darktable-cli reads darktablerc.

**The engine never opens a folder or reads a file of settings.** The binaries read the store and hand it plain options.

- **The CLI** reads its user data folder's engine and export settings and never writes settings. It finds the folder with `Folders::find_to_read`, which creates and probes nothing, and its store is `Store::read_only`. A test runs it on files of every kind, damaged and newer included, and finds each byte for byte as it was, nothing set aside and no temporary file removed. Only the commands that run the engine (`gpu`, `preview`, `export`) open the store.
- **The app** opens one store on its registry and the engine's, the export's and the tools' settings (`tools.*`) together, so their keys share its file and are never unknown to it.
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
- **`-d settings`** logs the folder and how it was chosen, then the cache folder and how it was chosen.

**One whole folder on Linux,** as darktable's `~/.config/darktable`, VS Code's and Chromium's are, rather than XDG's split by kind (settings in `$XDG_CONFIG_HOME`, the rest in `$XDG_DATA_HOME`, Open Recent and reports in `$XDG_STATE_HOME`): one place to find and back up, laid out as a chosen `--user-data-dir` is. Windows keeps its `config` subfolder.

**What it holds:** the person's settings and personal resources.

- `settings.toml` and `state.toml`, and while the app starts, `starting.toml`, the marker of a start under way, locked by it (decision 82);
- `instance.lock`, held by the app's instance on the folder for its whole run, holding nothing else (decision 89);
- `keywords.toml`, the keywords' properties and Recent, shared by every folder and Library (planned, decisions 73 and 101);
- `workspaces/<id>.toml`, one file per workspace: a person's own, and Default's or Essentials' once changed, each with its own `version`, read as the app starts, a person's own then listed in View › Workspace (decision 96; [gui.md](gui.md#workspaces)); written whole through the safe write once a person changes it (step 4);
- `themes/` (decision 36);
- planned, what clearing the cache must not lose (decisions 76 and 101): `folders/<id>.toml`, the folder records, each with `folders/<id>/pending/`, its changes waiting while its drive is away, and `folders/<id>/edits/`, its edits where they are kept on this computer; `libraries/<id>/`, each Library kept on this computer, its `library.toml`, `albums/` and `smart-albums/`, or, for a Library saved in a folder, its home folder's record and the last copy read; `last-written/`, the last copy of each sidecar this machine wrote, for silent winners; and `conflict-copies/`, conflict copies kept 30 days once compared (decision 52);
- later scripts and similar personal resources, `styles/`, `presets/` and `reports/` (crash reports and the log file; planned with the crash report, which replaces the home folder with `~` and photo and folder names with placeholders, keeping file extensions and camera models, while the local log file keeps full paths, decision 44). Export presets come first: `presets/export.toml`, a table per preset keyed by its name and holding the `export.*` settings' keys, read with the settings' rules, written by the app through the store's merge-write and read by `apertrust-cli export --preset NAME` (decision 56, planned).

No other platform folder is used, XDG's data and state folders among them, so one folder holds a person's whole set-up; the cache is apart, below.

**The photos do not live there, nor their sidecars, but for a folder that keeps its edits on this computer.** The user data folder holds what this computer knows of them: the folder records, the Libraries kept on this computer, lists of folders and their albums, and `keywords.toml` (above, [library.md](library.md#a-library)); a folder's own albums are in its `.apertrust/`, and each folder tree's index is in the cache folder (decision 55). Edits are beside the photos, or in a folder record's `edits/` for a folder whose edits are kept on this computer (decision 47).

**The cache folder** holds only what Apertrust can rebuild with nothing lost; its kinds, entries, limit and clearing are [cache.md](cache.md)'s (decision 76). What settings own of it:

- **Where:** `--cache-dir DIR` or `=DIR`, with darktable's `--cachedir` accepted as its alias, the last given, then `APERTRUST_CACHE_DIR`, empty counting as unset; with a user data folder given by flag or variable, `<user data>/cache`; otherwise the platform's cache folder, `~/Library/Caches/Apertrust`, `%LOCALAPPDATA%\Apertrust\cache`, `$XDG_CACHE_HOME/apertrust` (`~/.cache/apertrust`). Apertrust Nightly's is its own. Chosen by `Folders` in the app and the CLI alike, with `~` and a relative folder taken as for the user data folder, and never created or probed there ([cache.md](cache.md#the-folder)).
- **Its settings,** owner `engine`, on Settings › Storage, group Cache, applied at once: `cache.limited` (a boolean, on by default; off is Unlimited), `cache.limit` (GB of 10⁹ bytes, 0.5 to 1,000 in steps of 0.5, default 4, drawn disabled while `cache.limited` is off by the schema's `requires` field) and `cache.unused_after` (a choice of `apertrust_base::cache::UnusedAfter`, `days_30`, `days_90`, `days_180`, `days_365` or `never`, `days_90` by default). The engine's `settings::cache_limit` turns the first two into the cache's `Limit` ([cache.md](cache.md#the-limit-and-unlimited)).

**One app instance and one window per user data folder** ([decision 89](decisions/0089-one-instance-and-one-window-per-user-data-folder.md)), the folder compared by its canonical path; [gui.md](gui.md#instances-and-windows) has the whole design.

- **The lock:** the instance holds `instance.lock` in the folder for its whole run (`File::try_lock`), released by the system when it ends, a crash included.
- **A second start on the folder,** on Windows and Linux, hands its paths, already absolute, and its activation token to the instance through a named pipe on Windows (`\\.\pipe\apertrust-<hash>`) or an abstract Unix socket on Linux (`apertrust-<uid>-<hash>`), each only the user's own, `<hash>` the first 16 hex digits of BLAKE3 over the folder's canonical path, and exits. The instance opens them as an open from outside, replacing its window's place. On macOS, where LaunchServices delivers opens to the running app, there is no channel, and a second start is refused on stderr with exit status 1.
- **A start with flags for one run** (`--safe-mode`, `--conf`, `-d`, a `--cache-dir` naming another folder) while the instance runs is refused, with a message on stderr and exit status 1, and on Windows and Linux the instance's window brought forward, with an alert saying so on Linux, and on Windows once the app shows alerts there (planned): what was typed for the run is never ignored, and a folder never has two writers.
- **The CLI** takes no lock and reads the settings as it does now.
- **Another folder is another instance,** side by side with the first, of the same application: one app id, icon and file associations. On Windows and Linux opens from outside start with no `--user-data-dir`, so they reach the instance on the folder such a start uses, `APERTRUST_USER_DATA_DIR`'s where the desktop session sets it, else the platform's; on macOS an instance on another folder takes the open-documents events LaunchServices delivers to it. Its window's title ends with the folder's name, "Apertrust — work".
- Settings still merge per key, `state.toml` per change, and a workspace's file, written whole (decision 96's step 4), by applying the moves to it as it is on disk, so a change made to a file by hand meanwhile is kept.
- Two instances on different folders, or the app and `apertrust-cli library`, share a folder of photos through its files, with no lock but its index's own ([cache.md](cache.md#two-processes-at-once)): each writes whole files only when their bytes change, reads a file again before writing it if it changed, and sees the other's changes as any outside change (decisions 52 and 54). Each keeps its own cache unless both are given the same cache folder.

**Development builds** are Apertrust Nightly, which installs and runs beside Apertrust ([packaging.md](packaging.md#apertrust-nightly)), as nightly channels of other apps do.

- **Which builds are Nightly:** the nightly packages are always Nightly; the nightly job says so explicitly (`tools/package.sh --nightly`, which builds with `APERTRUST_NIGHTLY=1`). Every other build, from source or `cargo run`, stays Apertrust while no release tag exists, and becomes Nightly once one does: a build not on a release tag, when the history has one (`X.Y.Z+N~gHASH` or `-dirty`, decision 44). The rule is one condition where the version is computed (`crates/base/src/build/version.rs`); a packager's `APERTRUST_VERSION` is a release.
- **Its folder:** the platform's folder is Apertrust Nightly's, `~/Library/Application Support/Apertrust Nightly` on macOS, `%APPDATA%\Apertrust Nightly\config` on Windows and `$XDG_CONFIG_HOME/apertrust-nightly` on Linux, for the app and the CLI alike (`Folders::find(given, dev)`). A folder given by flag or variable is used as given. Its platform's cache folder is its own too: `~/Library/Caches/Apertrust Nightly`, `%LOCALAPPDATA%\Apertrust Nightly\cache`, `$XDG_CACHE_HOME/apertrust-nightly`.
- **Its name:** the window's title, the banner (`Apertrust Nightly 0.1.0+12~gabc1234 [...]`, `Apertrust Nightly CLI ...`) and the app's id, `org.apertrust.Apertrust.Nightly`.
- It separates a development build's settings only: its edits are the person's sidecars beside the photos, and a development build that understands an edit can still change it (below).

## Edits in another user data folder

- **Edits are beside the photos in every user data folder** (decisions 47 and 48), so `--user-data-dir` separates settings only, and so does Apertrust Nightly; `--help` and the docs say so.
- **Keep Edits on This Computer is one user data folder's** (planned, decision 101; [library.md](library.md#nothing-written-beside-the-raws)): a folder with the setting has its edits in this user data folder's record of it. **A known limit,** said here and in the setting's help: another computer, or another user data folder on this one (Apertrust Nightly, a development build), sees none of those edits and would write sidecars beside the photos, a second truth. `--conf library.keep_edits_here=true` sets it for every folder not yet edited, for one run, as for a test install.
- **The risk that remains,** stated plainly: a development build reads and writes the person's sidecars. The rule below stops it overwriting an edit that holds what it cannot keep, but a development build that understands an edit can still change it. Once it writes what the installed Apertrust lacks, a control the installed one does not have, say, the installed one opens that photo for viewing only.

**No build overwrites an edit a newer Apertrust wrote,** in any folder ([engine.md](engine.md), Edits, What a save may drop). Newer is told by what the edit holds, never by comparing builds' versions, which development builds on different branches do not order. It is anything in the edit that this build would drop on saving:

- a newer file version or tool version;
- a tool this build neither knows nor has retired;
- at a tool's own version: a control it lacks, or a value it cannot read or keep (an option it lacks, an option or control not ported, a number out of range);
- an operation it lacks, a schema version above its own, or at its own schema a parameter it lacks;
- an entry repeated, or any element or attribute it does not write.

Then:

- the photo opens for viewing only, rendered from what this build reads, its controls disabled, saying which version wrote it;
- a file version it cannot read at all refuses to open, naming the version;
- what it knows an older build wrote is dropped with a warning, and the photo stays editable: a tool Apertrust has retired, as the built Tone curve's `tone_curve` once it becomes RGB curve (decision 2), what an older tool or schema version held that this one lacks, a parameter a tool now owns, an optional operation's parameters, and version 0;
- every edit records the build that wrote it, decision 44's version rather than `Cargo.toml`'s.

**Versions rise for removals and changes, never for additions.** A tool's version, an operation's schema version and the file's version rise when what an older build wrote would read differently or not at all: a control, option or parameter removed or renamed, a range narrowed, a default changed, a meaning or binding changed, or a layout an older build would misread.

- The migration keeps the look. While no released edit can hold the value, it may instead drop it with a warning, as Exposure's version 2 will drop the Contrast and Pivot it held at version 1.
- An addition whose default keeps old edits' look raises nothing: a build without it finds it in the edit and opens the photo for viewing only.
- A raise for every addition, as the only guard, would not do: two branches' versions of the same number differ, so one branch's addition would be lost in the other's.

**Held by tests:**

- a committed record of each tool's stored controls and of the operation parameters an edit can store (those no tool owns, of operations an unedited photo runs). It fails a removal or change until the version rises, an addition until the record is blessed, and a tool or operation recorded but neither built in nor retired;
- a committed version-1 sidecar that reads with no warning and writes back byte for byte.

## The Settings dialog

Drawn on nine pages, as darktable's preferences: General, Import, Photos, Editing, Processing, Storage, Shortcuts, Presets and AI, Photos named so because "Library" names only a library you create (decision 37), its id `photos`; a `settings_window.page` of `library`, which earlier builds kept, is a value this build cannot read, so the dialog opens on General with a warning and the value stays until a page is chosen ([gui.md](gui.md#the-settings-dialog) holds its form, its rows and its window). It is a native window of its own (decision 41), on the Mac a modeless one rather than a sheet.

- **Rows from the registry:** each setting the schema gives a `page` and a `group` is a row on that page, under its group's title, in the schema's order, drawn by its kind. A row off its default has darktable's dot and is reset by a double-click on its label or its right-click menu's Reset; a row `--conf` sets is drawn disabled, saying so; "Applies after a restart" is said where the schema says so. Every change applies at once.
- **A list of a choice** (`kind = "list"`, `of = { kind = "choice", type = "…" }`) is a row of its own kind: a switch row per option of the type, stacked in the control column in the type's order, the row as tall as they are, with one dot and one Reset for the whole list; a change sets the list to the options ticked, in the type's order. It is generic, for any such list on a page; the options' words are `setting-<id>-option-<option>`, as a choice's.
- **A row's help** is its words' `setting-<id>-help`, where it has one, shown as its label's tooltip.
- **Interface,** General's first group: the theme, the language (`locale`), the interface's size (`scale`), the slider's marker (`sliders.marker`, decision 75) and, on Windows and Linux, the menu button (`toolbar.menu_button`, the app's icon, `icon`, by default, or ☰, `menu`; decision 39); planned, `interface.mode` (decision 94), Interface: Automatic, Standard or Touch, a choice of `automatic`, `standard` and `touch` applied at once, Standard by default on Windows, macOS and Linux and Touch on Android, Automatic picking Touch on a device with a touch screen and no pointer; View › Touch Mode switches it too ([gui.md](gui.md#touch-mode)).
- **Start,** General's second group: `library.reopen_last` (a boolean, on by default), whether the start reopens what was open last, which `state.toml` keeps ([`state.toml`](#the-settings-schema)), a folder, never a photo opened alone (decision 46), the start window showing where nothing is reopened ([gui.md](gui.md#the-start-window)); read once as the app starts, so a change counts from the next start, and kept by the dialog alone, never by the app's own settings.
- **Confirmations,** General's third group: `export.ask_before_overwrite` and `history.ask_before_discard` (booleans, on by default, applied at once), whether Export's overwrite question and Discard History… ask first ([gui.md](gui.md#built-on-the-mac-and-linux)); a yes with their alert's Don't ask again ticked turns each off.
- **Diagnostics,** General's fourth group: `log.keep` (a boolean, off by default, applied at once), whether the app keeps this session's log in memory for its log window, and `log.domains` (a list of `apertrust_base::log::Domain`, the `-d` domains but `golden`, then `verbose`; all but `memory` and `verbose` by default, applied at once), what it keeps besides warnings and errors ([gui.md](gui.md#the-log-window)). The list is kept while the log is off, and used once it is on.
- **Hand-built pages** for what rows cannot hold: Shortcuts, darktable's shortcuts page (decision 98); Storage, the user data and cache folders, each with how it was chosen, read-only, since a folder is chosen before the app starts, and the cache's sizes, settings and Clear ([cache.md](cache.md#settings--storage)); Presets; AI's models. Where a folder's edits are kept is that folder's setting in Collections' ⋯, with `library.keep_edits_here` for folders not yet edited (decisions 47 and 101).
- **Shortcuts,** planned (decision 98; [gui.md](gui.md#settings--shortcuts)): darktable's page, built by hand, with its settings beside it, each owner `app`, applied at once: Load default shortcuts at startup (`shortcuts.load_defaults`, on; darktable's `accel/load_defaults`, which darktable keeps under miscellaneous), off leaving the keymap the stored shortcuts alone, as `[keys]` below says, and never rewriting the file as it turns on or off; Enable fallbacks (`shortcuts.fallbacks`, off; `accel/enable_fallbacks`), the page's check box; and, kept with no row, the notice's dismissal (`shortcuts.hide_notice`; `accel/hide_notice`) the step's precision (`shortcuts.slider_precision`, normal, fine or coarse; `accel/slider_precision`), which an action cycles; and the speed adjustments (`[shortcuts.speeds]`, an action's id to its speed, darktable's speed-only shortcuts), which visual mapping's wheel sets, kept apart from `[keys]` so that setting one never replaces an action's shortcuts. The instance preferences come with layers.
- **`[keys]`** keeps an action's shortcuts where they differ from the built-in defaults, its list replacing that action's defaults and an empty list disabling them, as built. Planned (decision 98), it is always compared with the built-in defaults, whatever `shortcuts.load_defaults` says: on, the keymap is the defaults with each entry's list in its action's place, an entry equal to the defaults meaning what none means; off, it is the entries alone, an action with none having no shortcut. An entry equal to its action's defaults is left out when written, but one whose shortcuts carry `default = true`, darktable's `is_default`: a shortcut a person assigns while loading is off that equals its default is written so, and is there after a restart. A plain trigger is a string in Qt's portable form (`"Ctrl+Shift+Z"`); planned (decision 98), anything more an inline table of darktable's fields by darktable's words, `keys`, `press`, `button`, `click`, `move`, `direction`, `element`, `effect`, `speed`, `instance`, `device` and `default`, so the schema's `keys` takes list items of any TOML value, kept as written and parsed by the app. A file written now reads unchanged, and no version changes ([gui.md](gui.md#reading-and-writing)):

  ```toml
  [keys]
  show-log = ["Ctrl+Shift+L"]
  undo = [{ keys = "Ctrl+Z", default = true }]   # assigned while loading defaults was off
  tool-exposure-control-exposure = [{ keys = "E", move = "scroll" }, { keys = "Shift+E", effect = "edit" }]
  tool-toneequal = [{ keys = "T", press = "double", element = "show", effect = "on" }]
  fallbacks-value = [{ keys = "Ctrl", speed = 0.1 }, { keys = "Shift", speed = 10 }]   # in place of darktable's value fallbacks
  ```

- **A page that holds nothing yet** is listed dimmed and cannot be chosen. Built: General, Photos, Editing, Processing and Storage, its folders and the cache's contents built by hand beside its rows from the schema ([cache.md](cache.md#settings--storage)); Import, Shortcuts, Presets and AI are dimmed.
- **Import,** planned with step 20a (decision 85; [gui.md](gui.md#the-import-sheet)): darktable's four import preferences, Base folder as `import.default_folder` (`$(PICTURES_FOLDER)/Apertrust`), the default folder, first under the Folder ⌄'s New subfolders in, what the Import tool's ↺ puts back and the second tree the already-imported check walks, beside `import.base_folder`, this import's Folder, which the sheet changes, `import.subfolder` (`$(YEAR)$(MONTH)$(DAY)_$(JOBCODE)`), `import.file_name` (`$(YEAR)$(MONTH)$(DAY)_$(SEQUENCE).$(FILE_EXTENSION)`) and `import.keep_original_name` (off), texts and a boolean applied at once, where darktable keeps `session/base_directory_pattern`, one key for its preference and its dialog, `session/sub_directory_pattern`, `session/filename_pattern` and `session/use_filename`; then with step 20c `import.card_connected`, When a card is connected (Do nothing, Mark Import, the default, or Open Import). The sheet's other settings, darktable's `ui_last/import_*`, are drawn in the sheet alone, as darktable keeps them out of its preferences.
- **A row whose `requires` is off** is drawn disabled, as a row `--conf` sets is, with nothing said under it: the Storage page's Cache limit while Limit the cache's size is off.
- What the app remembers on its own (the window's placement, the sections open, the scopes' view, the tab shown, `state.toml`'s, the current workspace, `workspace`, Show Hidden Tools, `tools_panel.show_hidden`, the workspaces themselves each a file of its own in `<user data>/workspaces/` (decision 96; [gui.md](gui.md#workspaces)), and the floating panels' places, `tools_panel.floating` (decision 83, planned; [gui.md](gui.md#what-the-file-looks-like)), the Browser's position, size, sort, thumbnails and whether they show their captions, `browser.labels`, the browser bar's Labels, the Tools panel's side, `tools_panel.position` (`right`, the default, or `left`, app-wide; [gui.md](gui.md#its-position)), the page Settings last showed, `settings_window.page`, and the log window's place, its size and the folder its Save to file… last wrote to, `log_window.*`) is never shown, and a tool's settings are drawn in its tool.
- **Square thumbnails,** `browser.square_thumbnails` (a boolean, off by default, applied at once; [gui.md](gui.md#the-cell)), is the Photos page's row, under Thumbnails; `--conf browser.square_thumbnails=true` turns it on for a run.
- **Tools panel,** the Editing page's group: Show tab names, `tools_panel.tab_names` (a boolean, on by default), and Mouse wheel scrolls the Tools panel, `tools_panel.wheel_scrolls_panel` (a boolean, on by default, applied at once; [gui.md](gui.md#the-scroll-wheel-over-a-row)), darktable's `darkroom/ui/sidebar_scroll_default`, off by default there: on, a plain scroll scrolls the panel and Ctrl+scroll, Option+scroll on the Mac, changes the slider or choice row under the pointer; off, the wheel over a row changes it plain. `--conf tools_panel.wheel_scrolls_panel=false` turns it off for a run, its row then drawn disabled.
- **Keep edits on this computer,** `library.keep_edits_here` (planned with the library's step 28, decision 101): a boolean, off by default, for every folder not yet edited, its help saying the known limit ([Edits in another user data folder](#edits-in-another-user-data-folder)); its page and group placed with the step.
- **Raw thumbnails from,** `library.raw_thumbnails_from`, darktable's `plugins/lighttable/thumbnail_raw_min_level` ("for unaltered images, use raw file instead of embedded JPEG from size") with its options and default, is the Photos page's second row, under Thumbnails, its help saying what it does for photos with no edit and that edited photos always show their edit: owner `engine`, a choice of `apertrust_library::ops::RawThumbnails`, kept by darktable's own strings (`always`, `small`, `VGA`, `720p`, `1080p`, `WQXGA`, `4K`, `5K`, `6K`, `8K`, `never`, `auto`), `never` by default, applied at once. The app hands it to the session as it starts and as Settings changes it (`LibraryCommand::RawThumbnails`); what each option does is [library.md](library.md#thumbnails)'s.
- darktable's seven tabs hold lighttable and darkroom state that one view does not have, so they are not mirrored row for row: Apertrust mirrors darktable's modules, not its shell.
