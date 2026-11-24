# 0054. Four layers for the library's code

## Context

The library ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)) touches files, keeps a model of what is open, answers queries and serves the app, the CLI, MCP and later scripts. How is its code structured so that each part has one job, the abstraction stays small, and every client sees the same thing?

## Decision

**Four layers in `apertrust-library`, each calling only the one below; clients see only the last.**

| Layer | Its one job | Where |
| --- | --- | --- |
| 1. Files | Bytes in, values out and back: the `.apxmp` text layer with the comparison of two copies, over the XMP layer, and the library's TOML files (a folder's `.apertrust/` albums and smart albums, folder records, Libraries, `keywords.toml`); no folders listed, no threads | `sidecar`, `xmp` ([0050](0050-our-own-xmp-layer.md)), `files`, over `apertrust-base`'s `fs` |
| 2. Model | What is open, a folder tree or a Library ([0101](0101-folders-first-libraries-optional.md)), with its folders, photos, virtual copies, albums, smart albums and keywords; every change applied here, written through layer 1, reported as events | `model` |
| 3. Index | Rules, sorts, pages and full text: one `Index` type, in memory over the open tree, saved per folder tree in the cache ([0055](0055-the-librarys-cache.md)), a Library's search reading its folders' indexes together; always rebuilt from layer 2 | `index`, `listing` |
| 4. Operations | `LibraryCommand` (about forty) and `LibraryEvent` (about twenty) in the session protocol | `ops` |

Layer 1 alone knows a photo folder's file layouts; layer 3 alone the index's engine.

**Outside the four layers, `presets`:** the presets and styles of the user data folder ([0106](0106-presets-and-styles-as-appreset-and-apstyle-files.md)), files of no photo folder, which the engine never opens ([0043](0043-settings-and-apertrusts-folders.md)). Clients call it directly, as they read `settings.toml`; it uses layer 1's `xmp` and `sidecar`'s entry reading and writing and the pure `listing::browser_order`, and no layer calls it.

**A storage interface waits for a second backend.** The library calls the file system through `std::fs` and `apertrust-base`'s safe write, with the per-folder queues and their deadlines ([0053](0053-finding-folders-and-photos.md)) and the comparison before every write ([0052](0052-comparing-two-copies-of-a-sidecar.md)) above them. Should a second backend come, the server's later, if asked ([0071](0071-a-self-hosted-server-with-sync.md)), every call into a folder's files moves behind one interface under layer 1 and the listing (a folder listed in chunks, a file's facts, a file read whole or a byte range of it, the safe write, rename, move, copy and delete), so nothing above it changes.

**Layer 4 is served by the engine's session** ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)): the long commands run as jobs of its control on darktable's queues ([0057](0057-jobs-as-darktables-control-jobs.md)), a folder's reading `UserBackground`, thumbnails `SystemForeground`, Move to…, Copy to…, Rename, Delete and Import… `UserForeground`, each call into a folder tree on its queue. A job's progress and cancel are the session's `Event::Job` and `CancelJob`. Each `LibraryEvent` is a fact or a notice ([0059](0059-one-event-system.md)), sent through an observer the session wraps into `Event::Library`, carrying its command's number. The engine depends on this crate; the crate never depends on the engine, the tools, the plugin interface or wgpu.

**What a client sees:** `PhotoRef` (a folder record's id or a folder's path, a relative path and an optional virtual copy id: `5b0c7d0e/2026/Lisbon/DSCF0001.RAF#7c0e9a14`), `Target` (photos, the selection or a query) and `Rules` (darktable's collect rules, filters and sort, with "in album"). `Source` is `Folder` or `Photos` today, `Library` joining it in Phase 5. Commands come in five groups: open and organise (Libraries, their folders, albums); bring in (`Import`); browse; describe photos, ratings to snapshots and groups; files (`MoveTo`, `CopyTo`, `Rename`, `Delete`, `KeepAvailableOffline`, `KeepEditsHere`, `MakeAvailableOffline`). Events report every change whoever made it (`Opened`, `Listed` in chunks, `PhotosChanged`, `Diverged`, `FolderAway`, `InCloud`, `ViewOnly` among them). Undo and Redo stay the session's. Built so far: `Open(Source::Folder)` and `Close`, with `Opened`, `Subfolders`, `Listed` (chunks of 256), `Failed` and `Closed`.

**Left out on purpose:** SQL or any query language beyond darktable's rules; a trait for the index's engine, which has one implementation; a storage trait before a second backend; plug-in points until scripting has a second user; a sync engine beside [0052](0052-comparing-two-copies-of-a-sidecar.md)'s comparison, the server being one more copy; transactions across photos; a second API beside the protocol; undo in the library.

**Headless:** `apertrust-cli library` runs layers 2 to 4 in its process, with no lock between it and a running app: `scan` today, later `list`, `query --json`, `rate`, `label`, `tag`, `album`, `import`, `move`, `copy`, `check`, `rebuild`, `conflicts`. MCP tools are generated from `LibraryCommand`'s schema, read and write apart, off until the person allows them and read-only unless writes are allowed (principle 8). Scripting comes later as a protocol client, Rhai a candidate.

## Options considered

- The storage interface built before Phase 5: an abstraction with one implementation while the server is later, if asked; layer 1 and the listing stay small enough to move behind one when a second backend comes.
- A trait for the index's engine or for sync: an abstraction with one implementation each.
- An SQL API: a schema that becomes a public interface by accident, as darktable's did.
- A second API beside the protocol: two things for every client to keep in step.
- Separate crates for the sidecar and the file system: a crate with no boundary of its own to protect ([0065](0065-names-and-crates.md)).

## Consequences

- A new library feature is a command and its events in layer 4, a change in layer 2, and file format in layer 1 only if it is stored.
- The listing and the safe write move behind a storage interface only when a second backend is built.
- Lua through `mlua` would need an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).
