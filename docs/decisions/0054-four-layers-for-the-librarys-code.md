# 0054. Four layers for the library's code

## Context

The library ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)) touches files, keeps a model of what is open, answers queries and serves the app, the CLI, MCP and later scripts. How is its code structured so that each part has one job, the abstraction stays small, and every client sees the same thing?

## Decision

**Four layers in `apertrust-library`, each calling only the one below; clients see only the last.**

| Layer | Its one job | Where |
| --- | --- | --- |
| 1. Files | Bytes in, values out and back: the `.apxmp` text layer with the comparison of two copies, over the XMP layer, and the library's TOML files; no folders listed, no threads; every call into a root's files through the storage interface | `sidecar`, `xmp` ([0050](0050-our-own-xmp-layer.md)), `files`, over `storage` (planned) and `apertrust-base`'s `fs` |
| 2. Model | The open `Library` (a folder, a library or a project), its roots, photos, virtual copies, albums, smart albums and keywords; every change applied here, written through layer 1, reported as events | `model` |
| 3. Index | Rules, sorts, pages and full text: one `Index` type, in memory for folder mode, saved as a library's cache ([0055](0055-the-librarys-cache.md)); always rebuilt from layer 2 | `index`, `listing` |
| 4. Operations | `LibraryCommand` (about forty) and `LibraryEvent` (about twenty) in the session protocol | `ops` |

Layer 1 alone knows file layouts; layer 3 alone the cache's engine; the storage interface alone where the bytes are.

**The storage interface** (planned, `storage`) is the library's lowest part, under layer 1 and the listing, so no layer assumes a file system. Every call into a root's files goes through it: a folder listed in chunks, a file's facts, a file read whole or a byte range of it (the fingerprint's first and last 64 KiB, an embedded picture), the safe write, rename, move, copy and delete, and a raw fetched into a local copy. It has two backends:

- **local files:** `std::fs`, bulk listing, cloud placeholders' flags and `apertrust-base`'s safe write, as built today;
- **the server** ([0071](0071-a-self-hosted-server-with-sync.md)): a server root's files through the server's protocol, its raws fetched once into local copies by fingerprint.

Above it nothing knows which backend serves a root: the per-root queues and their deadlines ([0053](0053-finding-folders-and-photos.md)) and the comparison before every write ([0052](0052-comparing-two-copies-of-a-sidecar.md)) are the same for both. It is to be in place before Phase 5 builds on the file system (the library's step 21b), so the server's backend slots in beside the local one with no layer above it changing.

**Layer 4 is served by the engine's session** ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)): the long commands run as jobs of its control on darktable's queues ([0057](0057-jobs-as-darktables-control-jobs.md)), a folder's reading `UserBackground`, thumbnails `SystemForeground`, move, copy, rename, delete and Import… `UserForeground`, each call into a root on that root's queue. A job's progress and cancel are the session's `Event::Job` and `CancelJob`. Each `LibraryEvent` is a fact or a notice ([0059](0059-one-event-system.md)), sent through an observer the session wraps into `Event::Library`, carrying its command's number. The engine depends on this crate; the crate never depends on the engine, the modules, the plugin interface or wgpu.

**What a client sees:** `PhotoRef` (a root id or a folder path, a relative path and an optional virtual copy id: `5b0c7d0e/Photos/Wedding/DSCF0001.RAF#7c0e9a14`), `Target` (photos, the selection or a query) and `Rules` (darktable's collect rules, filters and sort, with "in album"). Commands come in five groups: open and organise; bring in and take out (`Import`, `ImportLibrary`, `ExportAsProject`); browse; describe photos, ratings to snapshots and groups; files (Move to `MakeAvailableOffline`). Events report every change whoever made it (`Opened`, `Listed` in chunks, `PhotosChanged`, `Diverged`, `RootOffline`, `InCloud`, `ViewOnly` among them). Undo and Redo stay the session's. Built so far: `Open(Source::Folder)` and `Close`, with `Opened`, `Subfolders`, `Listed` (chunks of 256), `Failed` and `Closed`.

**Left out on purpose:** SQL or any query language beyond darktable's rules; a trait for the cache's engine, which has one implementation; plug-in points until scripting has a second user; a sync engine beside [0052](0052-comparing-two-copies-of-a-sidecar.md)'s comparison, the server being one more copy; transactions across photos; a second API beside the protocol; undo in the library.

**Headless:** `apertrust-cli library` runs layers 2 to 4 in its process, with no lock between it and a running app: `scan` today, later `list`, `query --json`, `rate`, `label`, `tag`, `album`, `import`, `import-library`, `export-project`, `relink`, `check`, `rebuild`, `conflicts`. MCP's tools are generated from `LibraryCommand`'s schema, read and write apart, off until the person allows them and read-only unless writes are allowed (principle 8). Scripting comes later as a protocol client, Rhai a candidate.

## Options considered

- Layer 1 on `std::fs` alone: the server's backend would mean rewriting every call into a folder.
- A trait for the cache's engine or for sync: an abstraction with one implementation each.
- An SQL API: a schema that becomes a public interface by accident, as darktable's did.
- A second API beside the protocol: two things for every client to keep in step.
- Separate crates for the sidecar and the file system: a crate with no boundary of its own to protect ([0065](0065-names-and-crates.md)).

## Consequences

- A new library feature is a command and its events in layer 4, a change in layer 2, and file format in layer 1 only if it is stored.
- The listing and the safe write move behind the storage interface before Phase 5's library builds on them.
- Lua through `mlua` would need an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md).
