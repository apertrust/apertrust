# 0076. The cache: kinds, limits and control

## Context

Apertrust keeps on disk what it can compute again: thumbnails first (the library's step 6), then each library's cache ([0055](0055-the-librarys-cache.md)), local copies of raws ([0053](0053-finding-folders-and-photos.md), [0071](0071-a-self-hosted-server-with-sync.md)), later compiled pipelines, previews and AI results. darktable's disk cache has no lifecycle: its mipmaps grow without limit, and nothing shows their size or clears them. Apertrust's must show its size, take a limit, and be cleared by the person; it grows without bound only when the person chooses so.

It is read and written by the app, a second instance and the CLI at once, with no lock in the person's folders ([0053](0053-finding-folders-and-photos.md)). The engine and the library both use it, and the library never depends on the engine ([0065](0065-names-and-crates.md)). The engine's RAM and GPU caches, under its memory budget, are another thing and not covered here.

## Decision

**One cache folder, split by kind, holding only what Apertrust can rebuild with no one's help and nothing lost.** Built: base's `cache`, the folder's choice with `--cache-dir` (or `--cachedir`) and `APERTRUST_CACHE_DIR`, the settings `cache.limited`, `cache.limit` and `cache.unused_after`, the app opening the cache and handing it to its session, the session's start walk and eviction, and the thumbnails kind for the camera's embedded pictures, keyed by the photo's path until the fingerprint is built; the session's `Sizes` and `Clear { kind }` with `Event::Cache`, and the CLI's `cache info` and `cache clear [KIND...]`. The rest is planned, built with the library's steps 6 and 6b, and the libraries' caches, their listing and their Remove, with step 27.

**The folder** ([0043](0043-settings-and-apertrusts-folders.md)): `--cache-dir DIR` (or darktable's `--cachedir`), then `APERTRUST_CACHE_DIR`, then `<user data>/cache` when the user data folder is given by flag or variable, then the platform's: `~/Library/Caches/Apertrust`, `%LOCALAPPDATA%\Apertrust\cache`, `$XDG_CACHE_HOME/apertrust`; Apertrust Dev's its own. `Folders` chooses it with the user data folder, the CLI finding it to read as it finds that one. A program that writes the cache creates the folder as it opens it, writing and removing a file of its own there (`Cache::create`), the cache's first write: the app as it starts; the CLI, whose commands write no cache yet, not at all. **A folder that cannot be created or written** is warned of once, as the program starts, and the program runs with no disk cache: it never refuses to start for it, and never warns again for each write it would have made.

**The kinds,** a closed list in base's `cache::KINDS` (each an id, its folder and its policy), as the settings schema is one list:

| Kind | Folder | Policy | Owner | Entry |
| --- | --- | --- | --- | --- |
| Thumbnails | `thumbnails/` | limited | the library | one per photo, virtual copy and size |
| Pipelines (if measured worthwhile) | `pipelines/` | limited | the engine | wgpu's pipeline cache, one per adapter (Vulkan only) |
| Library caches | `libraries/<library id>/` | kept | the library | one per library ([0055](0055-the-librarys-cache.md)) |
| Local copies | `copies/` | kept | the library | one raw per fingerprint, for offline drives and server roots |
| Later: previews, AI results such as depth maps | their own | limited | their owners | |

- **Limited** kinds count together against one limit and are removed oldest last use first, unless the person chooses an unlimited cache.
- **Kept** kinds are never removed by size: only by the person, or by the rule of their owner's step.
- **Thumbnails are stored as JPEG at quality 90,** through the JPEG encoder export already uses, chroma halved across (4:2:2), and read back by rawler's decoder, so none is added.
- Embeddings for search live inside a library's cache ([0055](0055-the-librarys-cache.md)). AI models are not cache: they are the AI page's downloads, placed with the AI work. Drivers' own shader caches (macOS's, Mesa's, the GPU vendors') are outside Apertrust's folder and not counted.

**One file per entry, the file system as the index.**

- **A limited entry is one file,** named by the hash of its slot, the slot being what it stands for (a thumbnail's: the photo's fingerprint, its virtual copy and the size, one of 128, 256, 512 or 1024), in 256 subfolders by the name's first byte. The file holds its version (a thumbnail's: the edit's hash and the render version, or the embedded picture's mark).
- **A write replaces the slot's file whole,** through a hidden temporary and a rename (base's `fs`). So a newer version removes the stale one in the same rename, and stale entries never pile up. A reader whose wanted version differs from the file's renders again, and may show the stale picture meanwhile.
- **Last use is the file's modified time,** which Apertrust sets as it writes the entry and as it reads it, at most once a day an entry. Access times are never read: many systems record them lazily or not at all, Windows on large volumes by default.
- **Sizes come from a walk,** one listing per subfolder with each file's size and time. Measured on the M4's internal SSD, warm, by a script: 0.2 s for 100,000 files; 10,000 removed in 0.56 s. The session walks the limited kinds once as it starts and keeps each kind's entries and bytes in memory, counting its own writes and removals, so the size shows at once. Another process's writes are counted at the next walk, which each eviction and each look at the Storage page runs. A tidy of its own, as the session starts, removes temporaries more than a day old, by name, as a folder's listing does; a walk writes nothing.
- **No index file and no lock** for limited kinds. Two processes writing one slot each rename a whole file, the last rename winning; a removal that finds the file gone is ignored; a reader whose file vanished renders again. A crash leaves at most a temporary, which the start's tidy removes.
- **A kept entry is a folder or a file of its own.** A library's cache records the library's name, its last path and when it was last opened on this machine. A process that has one open holds an OS lock on its `lock` file (`std::fs::File::lock`), so no clear removes it under that process.

**The limit,** two settings, owner `engine`, on Settings › Storage, group Cache, applied at once, each settable for one run by `--conf`:

- `cache.limited`, a boolean, on by default: whether the limited kinds are kept within a limit. Off is Unlimited: nothing is evicted by size, and the Storage page and `cache info` still show every size and offer Clear.
- `cache.limit`, a number in GB from 0.5 to 1,000 in steps of 0.5, default 4: the limit, its row drawn disabled while `cache.limited` is off, by a schema field `requires = "cache.limited"` that names the boolean a row depends on, added with it. 4 GB holds about 61,000 thumbnails at 512 px or 20,000 at 1024 px, at 66 kB and 200 kB each, measured on the corpus's embedded pictures.

**Time:** nothing in the cache is removed by age. `cache.unused_after`, owner `engine`, a choice of `apertrust_base::cache::UnusedAfter` (`days_30`, `days_90`, `days_180`, `days_365`, `never`), default `days_90`, on the same page and group, is how long a library's cache may go unopened on this machine before it is marked and offered for removal; with `never` none is offered for its age.

**With Unlimited and Never, nothing in the cache is ever removed except by the person's Clear or Remove** (and by a write replacing its own slot's stale version). darktable's disk cache grows without bound too; the difference is that here unlimited is the person's choice, not the default, and its size is always shown and can be cleared.

**Eviction,** a `SystemBackground` job of kind `cache-evict`, with no progress and never shown, one at a time (none added while one is queued or running):

- while `cache.limited` is on: run by the start's walk where the cache is past the limit; added after a write that takes the limited kinds past the limit, and when the limit is lowered or turned on (`Command::Cache(CacheCommand::Limit(limit))`, which the app sends as either setting changes); never while it is off, nor when the limit is raised;
- a request while one is queued is that one's, which reads the limit as it runs; a request while one runs, the start's walk counting as one, is folded into it, which walks and evicts once more before it ends; one that panics leaves the next request free;
- it walks the limited kinds, then removes their entries oldest last use first, across kinds, until they hold at most 90% of the limit, so it does not run again at the next write;
- its cost is the walk and the removals, 0.2 s for 100,000 entries and 0.06 ms a removal on the M4, off the engine thread and the interface's, and paused by an export as darktable's crawler is ([0057](0057-jobs-as-darktables-control-jobs.md)), which is planned; as the session ends it stops before its next subfolder or removal, as the walk and the tidy do, so quitting never waits for a cold walk.

**The start's walk,** a `SystemBackground` job of kind `cache-walk`, with no progress and never shown, added as the session starts: the tidy of old temporaries and of the write checks `Cache::create` left, then the walk, then eviction in the same job where the limited kinds are past the limit. The walk's sizes and time and each eviction's removals, failures and time are `-d perf` lines.

**Kept kinds' lifecycle:**

- **A library's cache** is never removed by size. One whose library has not been opened on this machine for `cache.unused_after` (90 days by default, never with `never`), or was not found at its last path when last looked for, is marked on the Storage page and offered for removal; it is never removed without the person. The open library's cache is rebuilt by Rebuild Cache, not removed.
- **Local copies** are never removed by size; how they are cleared, and when one is no longer needed, is decided with the library's step 31.

**Not in the cache:** what would be lost, or need the person, if the cache were cleared. It lives in the user data folder ([0043](0043-settings-and-apertrusts-folders.md)), which the person backs up, and nothing clears it with the cache:

- `<user data>/libraries/<library id>/`: the library's root map on this machine ([0053](0053-finding-folders-and-photos.md)), whose loss would ask the person to locate roots again; the last copy of each sidecar this machine wrote, for silent winners ([0052](0052-comparing-two-copies-of-a-sidecar.md)); a server root's sidecars with the changes the server has not taken yet ([0071](0071-a-self-hosted-server-with-sync.md));
- `<user data>/conflict-copies/`: conflict copies, moved there once compared ([0052](0052-comparing-two-copies-of-a-sidecar.md)), each the only copy of what it held, removed 30 days later by a `SystemBackground` job as the app starts.

**Control in the app,** Settings › Storage, a hand-built page with one row from the schema:

- the cache folder, how it was chosen, and Show in Finder (Explorer, the file manager);
- each kind present, with its entries and its size, and Clear;
- the limit, `cache.limited`'s and `cache.limit`'s rows, with the limited kinds' total against it, or alone while unlimited;
- `cache.unused_after`'s row;
- each library's cache with the library's name, its last path, when it was last opened and its size, those unused for `cache.unused_after` or not found marked, each with Remove;
- the conflict copies' count and size, with Show in Finder, and no Clear.

Clear and Remove are a `UserBackground` job of kind `cache-clear` with a progress, cancellable, as Rebuild Cache is ([0057](0057-jobs-as-darktables-control-jobs.md)), removing the entries listed as it starts; a library cache another process has open is skipped and said so. The page reaches the cache through the session: `Command::Cache`, its `CacheCommand` holding `Limit`, `Sizes` and `Clear { kind }`, and with step 27 `Remove { library }`, and `Event::Cache`, a fact holding each kind's sizes, and with step 27 each library's cache, their types in base's `cache` as the jobs' events are in its `jobs`.

**The CLI,** with no session and no GPU, calling base's `cache` directly, as `library scan` builds an index in its own process:

- `apertrust-cli cache info`: the folder and how it was chosen; the limit, or `unlimited`, and the limited kinds' total; `cache.unused_after`; each kind's entries and size; each library's cache with its id, name, last path, last opened and size, marked as on the Storage page. It walks and writes nothing.
- `apertrust-cli cache clear [KIND...] [--library ID]`: no kind clears every limited kind; `libraries` every library's cache no process has open; `--library ID` one. Local copies wait for step 31. It prints what it removed and what it skipped.
- `--cache-dir DIR` (or `--cachedir`), global, as `--user-data-dir` is.

**Where the code lives:** `apertrust_base::cache`, behind a `cache` feature: the kinds, the folder's layout, the entry's write, read and touch, the walk, eviction's order, clearing, and the commands' and events' types. Owners name their slots and versions: the library's thumbnails, library caches and local copies, the engine's pipelines. The engine opens no folder ([0043](0043-settings-and-apertrusts-folders.md)): a binary opens the cache and hands the session its handle with the limit (`CacheOptions`), the golden harness none; the engine's `session::cache` runs the start's walk and eviction.

## Options considered

- **A per-kind index of entries** (key, size, last use), a snapshot with one append-only journal per process and compaction under OS locks: size known without a walk, but a format, a compaction and a recovery to build and test, where a walk of 100,000 entries takes 0.2 s and runs in the background. Kept as the next step should a walk on Windows or a hard disk prove too slow.
- **One index file rewritten under a lock:** about 8.6 MB at 100,000 entries, rewritten for every batch of last uses while browsing.
- **An embedded database:** redb opens a file in one process only, so the app and the CLI could not share it; SQLite is C ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).
- **The file system's access times for last use:** off or lazy on many systems.
- **A file per version,** stale ones removed after a newer write: a listing per write, where one file per slot makes the rename remove it.
- **The cache in `apertrust-library`:** the engine depends on the library, so the direction holds, but the engine's pipelines would sit in the library without the GPU, and the CLI's `cache` and the server's index would pull in the library.
- **A crate of its own, `apertrust-cache`:** none of [0065](0065-names-and-crates.md)'s three reasons applies.
- **The cache in the engine:** the library may not depend on it.
- **A limit for each kind:** the person manages one amount of disk; previews and AI results join the same limit, oldest first.
- **Library caches counted against the limit:** they cannot be evicted, so they would push thumbnails out to make room.
- **Unlimited as the default, as darktable's:** the cache would grow without bound unless the person acts.
- **No unlimited choice:** a person with room to spare, or a cache on a disk of its own, would see thumbnails rendered again for nothing.
- **One number with a value for unlimited (0, say):** a number's range would hold a value that means something else; the boolean says it in the dialog and in `settings.toml`.
- **The unused time as a number of days:** the same, with a value for never.
- **The root map, last-written sidecars, pending changes and conflict copies in the cache:** clearing it would lose edits or ask the person to locate roots again.

## Consequences

- [0055](0055-the-librarys-cache.md): a library's cache is the cache's kept kind `libraries`; its root map and last-written sidecars move to the user data folder.
- [0052](0052-comparing-two-copies-of-a-sidecar.md) and the library's step 13: conflict copies kept 30 days in the user data folder; silent winners' copies there too. [0071](0071-a-self-hosted-server-with-sync.md): a server root's sidecars and pending changes there, its raws' local copies in the cache.
- [0043](0043-settings-and-apertrusts-folders.md): the cache folder decided; the user data folder holds each library's per-machine state.
- `apertrust-base` gains `cache` and the cache folder in `Folders`; the schema `cache.limited`, `cache.limit`, `cache.unused_after` and its `requires` field; the session `Command::Cache` and `Event::Cache` and the eviction and clear jobs; the CLI `cache info` and `cache clear`; Settings' Storage page its cache part.
- The library's step 6 builds the folder, the thumbnails kind, the limit and eviction; step 6b the Storage page and the CLI's commands.
