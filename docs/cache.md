# The cache

Apertrust keeps on disk what it can compute again, so that a folder's thumbnails, a library's index and later previews come back at once instead of being made again. This document describes the cache: where it lives, what it holds, how it stays within its limit, and how the person sees and clears it. The settings it reads are in [settings.md](settings.md), the thumbnails and the library's caches themselves in [library.md](library.md).

**Status.** [Decision 76](decisions/0076-the-cache-kinds-limits-and-control.md) records the design, with the options considered. Base's `cache` module is built: the kinds, a limited entry's write and read with its last use, the walk, the tidy of old temporaries, eviction past a limit to 90% of it and clearing a limited kind, on a folder its caller gives. So are the folder's choice in `Folders`, with `--cache-dir` (or darktable's `--cachedir`) in the app and the CLI, and the three settings, on Settings › Storage as rows, with the engine's `settings::cache_limit` turning the first two into a `Limit`. The app opens the cache as it starts and hands it to its session, which tidies and walks it as it starts and keeps it within the limit by eviction, as background jobs ([Through the session](#through-the-session)). The thumbnails kind is built for the camera's embedded pictures: read from the cache first, written to it on a miss ([Thumbnails](#thumbnails)). The rest is planned: Apertrust's own rendered thumbnails with the library's step 6, the photo's fingerprint as their slot with step 8, the session's other cache commands and events, the Storage page's hand-built part and `apertrust-cli cache` with step 6b, library caches with step 27 and local copies with step 31 ([roadmap.md](roadmap.md#the-librarys-steps)).

## What goes in it

The cache holds only what Apertrust can rebuild with no one's help and nothing lost. Clearing all of it costs time, never an edit, a rating or a question to the person. What would be lost goes in the user data folder instead ([What is not in the cache](#what-is-not-in-the-cache)).

## The folder

Chosen once, as the program starts, by `Folders` in `apertrust-base` ([settings.md](settings.md#user-data-folders)), in this order:

1. `--cache-dir DIR`, or darktable's `--cachedir` as its alias, as `--configdir` is `--user-data-dir`'s;
2. `APERTRUST_CACHE_DIR`;
3. `<user data>/cache`, when the user data folder is given by flag or variable, since a development build changes kernels without bumping the thumbnails' render version and could otherwise show its thumbnails in the person's Apertrust;
4. the platform's cache folder: `~/Library/Caches/Apertrust` on macOS, `%LOCALAPPDATA%\Apertrust\cache` on Windows, `$XDG_CACHE_HOME/apertrust` (`~/.cache/apertrust`) on Linux.

Apertrust Dev has its own. A leading `~` and a relative folder are taken as for the user data folder, and one given empty, or with a `~` and no home folder, stops the start. `Folders` only chooses the folder, never creating or probing it, and `-d settings` logs it with how it was chosen; the CLI finds it as the app does.

**Opening it.** A program that writes the cache opens it with `Cache::create`, which creates the folder where it is missing and writes and removes a file of its own there: the cache's first write. The app does so as it starts; the CLI's commands write no cache yet, so it opens none. A folder that cannot be created or written is warned of once, as the program starts, and the program runs with no disk cache: it never refuses to start, and is never warned of again for each write it would have made.

## Kinds

Each kind is a folder of its own, listed in `apertrust_base::cache::KINDS` with its policy.

```mermaid
flowchart TD
    cache["Cache folder"]
    cache --> limited["Limited: one limit together,<br/>oldest last use removed first"]
    cache --> kept["Kept: never removed by size or age"]
    limited --> thumbs["thumbnails/<br/>the library"]
    limited --> pipes["pipelines/<br/>the engine, if measured worthwhile"]
    limited --> later["later: previews, AI results"]
    kept --> libs["libraries/&lt;library id&gt;/<br/>the library"]
    kept --> copies["copies/<br/>the library"]
```

| Kind | Folder | Policy | Owner | An entry |
| --- | --- | --- | --- | --- |
| Thumbnails | `thumbnails/` | limited | the library | a photo, virtual copy and size |
| Pipelines | `pipelines/` | limited | the engine | wgpu's pipeline cache for one adapter, Vulkan only, if measured worthwhile |
| Library caches | `libraries/<library id>/` | kept | the library | one library ([A library's cache](library.md#a-librarys-cache)) |
| Local copies | `copies/` | kept | the library | one raw by fingerprint, for offline drives and server roots |
| Later: previews, AI results such as depth maps | their own | limited | their owners | |

- **Limited** kinds count together against one limit and lose their oldest entries first, unless the person makes the cache unlimited.
- **Kept** kinds are removed only by the person, or by their own step's rule.
- **Thumbnails** are stored as JPEG at quality 90, through the JPEG encoder export uses ([Thumbnails](#thumbnails)).
- **Elsewhere:** embeddings for search live inside a library's cache (decision 55). AI models are the AI page's downloads, not cache. Drivers' own shader caches (macOS's, Mesa's, the GPU vendors') are outside Apertrust's folder and not counted.

## Entries

- **One file per slot.** A limited entry's slot is what it stands for: a thumbnail's is the photo's fingerprint, its virtual copy and the size, one of 128, 256, 512 or 1024; until the fingerprint is built (step 8), the photo's path stands for it. The file is named by the slot's hash, in 256 subfolders by the name's first byte.
- **The version inside.** The file holds its version: a thumbnail's is the edit's hash and the render version, or the embedded picture's mark, which is how the picture was read (`apertrust_raw::PREVIEW_VERSION`, rawler's version and the reading's own) and the raw's size and modified time. A reader wanting another version renders again, and may show the stale picture meanwhile.
- **A write replaces the slot's file whole,** through a hidden temporary and a rename (base's `fs`), so a newer version removes the stale one at once and stale entries never pile up.
- **Last use is the file's modified time,** which Apertrust sets as it writes the entry and as it reads it, at most once a day an entry. Access times are never read: many systems record them lazily or not at all.
- **A kept entry** is a folder or a file of its own. A library's cache records the library's name, its last path and when it was last opened on this machine.

## Sizes and the walk

There is no index file. Sizes come from a walk of each limited kind: one listing per subfolder, with each file's size and time.

- **Its cost:** 0.2 s for 100,000 files on the M4's internal SSD, warm; 10,000 removed in 0.56 s. Measured again with thumbnails written by the session's own path: 164 to 248 ms for 100,000 entries, the first walk of each process the slowest ([below](#timing-the-walk)).
- **When:** once as the session starts, then before each eviction and each time the Storage page asks. In between, the session counts its own writes and removals, so the size shows at once; another process's writes are counted at the next walk.
- **Tidying:** temporaries more than a day old are removed by name, as a folder's listing does, by a tidy of their own as the session starts, with the write checks `Cache::create` left in the cache folder; a walk writes nothing.
- **Stopping:** the walk, the tidy and eviction's listing ask their cancel before each subfolder, and eviction before each removal, so a program that quits never waits for a cold walk; a walk stopped keeps no sizes.

## Thumbnails

Built for the camera's embedded pictures, the only thumbnails until Apertrust renders its own ([library.md](library.md#thumbnails)). A thumbnail job (`LibraryCommand::Thumbnails`), given a cache:

- **reads the cache first,** by the slot and version above (`ops::thumbnail_key`), and sends the picture as the library would;
- **on a miss** reads the embedded picture and sends it as without a cache, then encodes it and writes it, so a write never delays the picture shown. A write past the limit asks for an eviction;
- **keeps nothing** for a raw with no embedded picture, or a picture that cannot be encoded: it is sent, and read again the next time;
- with no cache, in the CLI or where the folder cannot be written, reads as before.

**The JPEG:** quality 90, through the export's JPEG writer (`apertrust_export::jpeg::write`), with chroma whole (4:4:4). It is read back by rawler's own decoder (`apertrust_raw::read_jpeg`), the image crate's zune-jpeg, so no decoder is added; zune-jpeg misreads the export's subsampled chroma with its optimised tables, which 4:4:4 avoids. The encoding runs in the engine, where the thumbnail jobs run: the library depends on neither the export nor the GPU, and needs neither for this.

**Apertrust's own rendered thumbnails** are planned with step 6: a still of the viewport pipe, its version the edit's hash and the render version.

### Timing the walk

An ignored test, `session::thumbnail::tests::time_the_walk`, measures it:

```sh
APERTRUST_CACHE_TIMING=DIR cargo test --release -p apertrust-engine --lib \
    time_the_walk -- --ignored --nocapture
```

On its first run in `DIR`, on the disk to time, it writes the thumbnails of every raw of the corpus (`APERTRUST_RAW_CORPUS` or `test-images/raw.pixls.us`) at 256, 512 and 1024 through the session's own path, reads them back as hits, and tells their sizes; then it fills the thumbnails kind to 100,000 entries. Every run then walks it five times. For a cold first walk, flush the file system's cache first: `sudo purge` on a Mac, a restart or a fresh mount elsewhere.

On the M4's internal SSD, 41 raws of the corpus, 33 with an embedded picture rawler reads:

| Size | Written, 41 asked | Read back | Each entry |
| --- | --- | --- | --- |
| 256 | 2.93 s | 0.03 s, 33 hits | 27.7 kB |
| 512 | 1.99 s | 0.05 s, 33 hits | 74.1 kB |
| 1024 | 2.31 s | 0.16 s, 33 hits | 226.7 kB |

The walk of 100,000 entries, warm, three processes of five walks each: 181.8 to 247.9 ms for the first walk of a process, 163.6 to 199.7 ms for the rest.

## Two processes at once

The app, a second instance and the CLI share the cache with no lock for limited kinds:

- two processes writing one slot each rename a whole file, the last rename winning;
- a removal that finds the file gone is ignored, and a reader whose file vanished renders again;
- a crash leaves at most a temporary, which the start's tidy removes.

A library's cache is locked: a process that has it open holds an OS lock on its `lock` file (`std::fs::File::lock`), and no clear removes it then.

## The limit and Unlimited

Two settings of owner `engine`, declared in the settings schema ([settings.md](settings.md#user-data-folders)):

- `cache.limited`, on by default. Off is **Unlimited**: nothing is evicted by size, and every size and Clear stay.
- `cache.limit`, in GB of 10⁹ bytes, as disks count them, from 0.5 to 1,000 in steps of 0.5, 4 by default, drawn disabled while `cache.limited` is off (the schema's `requires`). 4 GB holds about 54,000 thumbnails at 512 px or 17,600 at 1024 px, at 74 kB and 227 kB each, measured on the corpus's embedded pictures ([below](#timing-the-walk)); at 256 px, 28 kB.

Unlimited is the person's choice, never the default, and its size is always shown and can be cleared, where darktable's disk cache grows unseen.

## Eviction

While `cache.limited` is on, a `SystemBackground` job of kind `cache-evict`, with no progress and never shown, one at a time:

- **run** by the start's walk itself where the cache is past the limit, the walk counting as an eviction running; **added** after a write that takes the limited kinds past the limit, and when the limit is lowered or turned on; never when it is raised or turned off;
- **one at a time:** a request while one is queued is that one's, since it reads the limit as it runs; a request while one runs, the start's walk among them, is folded into it, which walks and evicts once more before it ends; one that panics leaves the next request free to add another;
- **walks** the limited kinds, then removes their entries oldest last use first, across kinds, until they hold at most 90% of the limit, so it does not run again at the next write;
- **costs** the walk and the removals, about 0.2 s for 100,000 entries and 0.06 ms a removal on the M4, off the engine thread and the interface's; it stops before its next removal as the session ends. Planned: paused by an export, as darktable's crawler is (decision 57).

## Time and Never

Nothing in the cache is removed by age.

- **A library's cache** unopened on this machine for `cache.unused_after` (30, 90, 180 or 365 days, or Never; 90 by default), or whose library was not found at its last path, is marked on the Storage page and offered for removal. It is never removed without the person, and the open library's is rebuilt by Rebuild Cache, not removed.
- **Local copies** are never removed by size or age; how they are cleared, and when one is no longer needed, is decided with the library's step 31.

**With Unlimited and Never, nothing in the cache is ever removed except by the person's Clear or Remove,** and by a write replacing its own slot's stale version.

## What is not in the cache

What clearing would lose, or would make the person do again, lives in the user data folder ([settings.md](settings.md#user-data-folders)), which the person backs up and nothing clears with the cache:

| What | Where | Why not the cache |
| --- | --- | --- |
| A library's root map on this machine (decision 53) | `<user data>/libraries/<library id>/` | its loss would ask the person to locate roots again |
| The last copy of each sidecar this machine wrote, for silent winners (decision 52) | `<user data>/libraries/<library id>/` | it may be the only copy of an edit a sync service overwrote |
| A server root's sidecars with the changes the server has not taken yet (decision 71) | `<user data>/libraries/<library id>/` | the changes are nowhere else |
| Conflict copies, once compared (decision 52) | `<user data>/conflict-copies/`, removed 30 days later | each is the only copy of what it held |

## Through the session

The engine opens no folder: a binary opens the cache and hands `Session::start` a `CacheOptions`, the `Arc<Cache>` and the limit its settings give, or none. The app hands one; the CLI, whose commands write no cache yet, hands none, and the golden harness starts no session.

**Built:**

- **As the session starts,** a `cache-walk` job: the tidy of old temporaries, then the walk, then eviction in the same job where the limited kinds are past the limit.
- **Eviction** ([above](#eviction)), asked for after a write past the limit (`Upkeep::check`, which the thumbnails' writes call), and by `Command::Cache(CacheCommand::Limit(limit))`, which the app sends each time Settings changes `cache.limited` or `cache.limit`, applied at once. Only the engine thread adds the jobs, since it owns the control: a worker asks it through the session's inbox.
- **The session's end** is the cancel of the tidy, the walk and eviction ([Sizes and the walk](#sizes-and-the-walk)), so quitting returns promptly.
- **Thumbnails** ([below](#thumbnails)), read from the cache by the thumbnail jobs and written on a miss.
- **Logging:** `-d perf`, the walk's sizes, the temporaries tidied and its time (each kind's under `verbose`), and each eviction's entries and bytes removed, its failures and its time; for each `LibraryCommand::Thumbnails`, once its last job has ended, how many came from the cache and how many were read, each with their time, and how many were written, each thumbnail under `verbose`; a cache that cannot be tidied, walked or evicted is a warning, as is a folder that cannot be created or written as the program starts.

**Planned** with step 6b ([session-protocol.md](session-protocol.md#commands-planned), [events.md](events.md)):

- **`Command::Cache(CacheCommand)`,** `Limit` built, and these variants beside it:
  - `Sizes`: walks the cache and answers;
  - `Clear { kind }`: removes the kind's entries listed as it starts;
  - `Remove { library }`: removes one library's cache, skipped and said so while another process has it open.
- **`Event::Cache`:** a fact for each kind (its entries and bytes) and for each library's cache.
- Their types are base's `cache`, as the jobs' events are its `jobs`.

**The jobs** (decision 57):

| Job | Queue | Shown | |
| --- | --- | --- | --- |
| `cache-walk`, the start's tidy and walk | `SystemBackground` | no | built |
| `cache-evict` | `SystemBackground` | no | built |
| `cache-clear`: Clear and Remove | `UserBackground`, cancellable, as Rebuild Cache | with a progress | step 6b |
| Conflict copies past 30 days removed, as the app starts | `SystemBackground` | no | step 13 |

## Settings › Storage

A hand-built page with rows from the schema ([gui.md](gui.md#the-settings-dialog)); a level the design draws from:

- the cache folder, how it was chosen, and Show in Finder (Explorer, the file manager);
- each kind present, with its entries and size, and Clear;
- the limit, `cache.limited`'s and `cache.limit`'s rows, with the limited kinds' total against it, or the total alone while unlimited;
- `cache.unused_after`'s row;
- each library's cache with the library's name, its last path, when it was last opened and its size, those unopened for `cache.unused_after` or not found marked, each with Remove;
- the conflict copies' count and size, with Show in Finder, and no Clear.

## The CLI

`apertrust-cli cache`, with no session and no GPU, calls base's `cache` directly, as `library scan` builds an index in its own process. `--cache-dir DIR`, or `--cachedir`, is global, as `--user-data-dir` is.

- **`cache info`:** the folder and how it was chosen; the limit, or `unlimited`, and the limited kinds' total; `cache.unused_after`; each kind's entries and size; each library's cache with its id, name, last path, last opened and size, marked as on the Storage page. It walks and writes nothing.
- **`cache clear [KIND...] [--library ID]`:** no kind clears every limited kind; `libraries` every library's cache no process has open; `--library ID` one. Local copies wait for step 31. It prints what it removed and what it skipped.

## Where the code lives

- `apertrust_base::cache`, behind a `cache` feature ([crates.md](crates.md#apertrust-base)): the kinds, the folder's layout and its creation (`Cache::create`), the entry's write, read and touch, the walk, eviction's order, clearing, `UnusedAfter`, and, planned, the commands' and events' types.
- The engine's `session::cache` ([crates.md](crates.md#the-cache)): `CacheOptions`, the start's job and eviction's; its `session::thumbnail`, the thumbnails read from and written to the cache.
- The library's `ops::thumbnail_key`, a thumbnail's slot and version.
- `apertrust-raw`'s `PREVIEW_VERSION` and `read_jpeg`, which reads a cached thumbnail back.
- The app's `open_cache`, which opens it as the app starts.

The CLI reads no thumbnails: `library scan` lists a folder, and no command of its reads or writes the cache yet.

Owners name their slots and versions: the library its thumbnails, library caches and local copies, the engine its pipelines.

## Open questions

- **The walk cold, on Windows and on hard disks:** the 164 to 248 ms for 100,000 entries are the M4's SSD with the file system's cache warm; a cold walk needs `sudo purge` first, which is the maintainer's to run. A cold walk on Windows, with its virus scanner, and on a hard disk are still to time, by the method [above](#timing-the-walk); a walk too slow for the start's background job brings in the per-kind index of entries decision 76 keeps as its next step.

- **Free space:** whether the cache also stops writing when its disk is nearly full, below the limit.
