# 0055. The saved index, one per folder tree

## Context

A large folder tree, or a Library of several, needs to open in under a second, browse folders that are away, search text and later find similar photos, which reading every sidecar each time cannot give. But no database may be the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), and a database on a share or in a sync folder breaks.

## Decision

**The saved index is one per folder tree per computer, disposable, never shared and never the truth** (planned, Phase 5, the library's step 23).

- **Per folder tree:** a folder opened with its subfolders has one index ([0101](0101-folders-first-libraries-optional.md)); a folder opened inside a tree that has one reads that tree's index. A Library's search and filters read its folders' indexes together; a folder in several Libraries has one index.
- **Place:** `<cache>/indexes/<tree id>/index-<schema>/` in the cache folder, the platform's or `<user data>/cache` when a user data folder is given ([0043](0043-settings-and-apertrusts-folders.md)), the tree's id its folder record's, else a hash of its path. The cache's kind `indexes` ([0076](0076-the-cache-kinds-limits-and-control.md)): **kept** while its tree's folder or any folder inside it has a record (in a Library, kept offline, or keeping its edits here), a recorded subfolder reading the enclosing tree's index, never removed by size and offered for removal once unopened for `cache.unused_after` (90 days by default, or never); **limited** for any other, an index whose last covered record is removed among them, counted against the cache's limit and removed oldest last use first. Never in the photos' folders, on a share or in a sync folder. The folder records, pending changes and the last-written sidecars, which clearing the cache would lose, are in the user data folder ([0076](0076-the-cache-kinds-limits-and-control.md)).
- **A folder opens without it.** It serves a large tree's speed, folders that are away, full-text search and later similarity; it needs no backup and never travels.
- **It holds,** as darktable's `images` table where it serves: each photo and virtual copy (folder, relative path, name, size, time, fingerprint, the sidecar's hash); EXIF (capture time, the camera's names as darktable's, lens, exposure, aperture, ISO, focal length, bias, white balance, flash, program, metering, size, aspect); rating, reject, labels, keywords, metadata, geotag, timestamps, group, the tools the edit sets, the history's length and the thumbnail's key.
- **Before it, in Phase 3,** the same facts in the same `Index` type in memory over the open tree ([0054](0054-four-layers-for-the-librarys-code.md)).
- **Built and rebuilt in the background:** by listing, sidecars and the raws' EXIF, the open folder first, in parallel, never reading photos in the cloud; again by Rebuild Index, or when missing or of another schema; later opens start from it, rescanning in the background, never blocking. At the measured SMB rates on the M4 against a NAS (342 ms for 2,825 files listed in bulk, 2.2 ms a sidecar read 16 at a time), 100,000 photos list in about 12 s and their sidecars read in about 4 minutes, EXIF at 35 ms a photo the long part.
- **A scan never drops a row:** a folder whose volume is not present is away, its rows kept; a file gone from a present volume is missing, re-paired by fingerprint; only the person removes a row ([0101](0101-folders-first-libraries-optional.md)).
- **Full text** over file names, keywords, title, description, notes, camera and lens, by darktable's text-search rule. Vectors for semantic search, similar photos and faces live only in the index, decided with the AI work.
- **Its engine is chosen by spike L2** (step 21), between plain Rust collections in memory with a snapshot file and `tantivy` (MIT) for full text, the likely choice, and Turso Database; `rusqlite` with SQLite compiled in only if both fail, and then only with an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md). L2 measures opening, a smart album of three rules, a full-text search, a rating set on 500 photos, a rebuild from cached sidecars and memory, at 100,000 and 500,000 synthetic photos, one tree of 500,000 on SMB built in the background with darktable's XMPs read, and five folders totalling 500,000 queried together, against a query under 50 ms and an open under 1 s at 100,000; plain collections cost about 1 kB a photo. Its result is recorded in this record.

## Options considered

- A database as the truth, or on shared storage: against [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), and broken by sync services.
- One index per library: a folder in two Libraries indexed twice, and no saved index for a folder in none.
- Every index kept: a tree opened once would hold its space until the person removed it.
- Turso preferred before it is measured.
- SQLite as the default: C compiled in.

## Consequences

- Nothing outside layer 3 knows the index's engine, so L2's choice changes one module.
- An index of another build's schema is rebuilt, never migrated. One whose folder is away is kept as it is until the folder can be read: meanwhile the folder shows as away with no photos listed, saying its index is from another version and is rebuilt once its drive is back; its offline thumbnails stay.
- The cache gains a kind whose entries are kept or limited by their folder ([0076](0076-the-cache-kinds-limits-and-control.md)).
