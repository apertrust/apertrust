# 0055. The library's cache

## Context

A large library needs to open in under a second, browse roots that are away, search text and later find similar photos, which reading every sidecar each time cannot give. But no database may be the truth ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), and a database on a share or in a sync folder breaks.

## Decision

**A library's cache is one per library per machine, disposable, never shared and never the truth.**

- **Place:** `<cache>/libraries/<library id>/cache-<schema>/` in the platform's cache folder, or `<user data>/cache` when a user data folder is given ([0043](0043-settings-and-apertrusts-folders.md)), the machine's root map and the last-written sidecars beside it; never in the library's folder, on a share or in a sync folder.
- **A library opens without it.** It serves a large library's speed, roots that are away, full-text search and later similarity; it needs no backup and never travels.
- **It holds,** as darktable's `images` table where it serves: each photo and virtual copy (root, folder, name, size, time, fingerprint, the sidecar's hash); EXIF (capture time, the camera's names as darktable's, lens, exposure, aperture, ISO, focal length, bias, white balance, flash, program, metering, size, aspect); rating, reject, labels, keywords, metadata, geotag, timestamps, group, local copy, the modules set, the history's length and the thumbnail's key.
- **Folder mode has none:** the same facts in the same `Index` type in memory ([0054](0054-four-layers-for-the-librarys-code.md)).
- **Rebuilt** by listing, sidecars and the raws' EXIF in the background, by Rebuild Cache, or when missing or of another schema; the open folder first, in parallel, never reading photos in the cloud. Estimated for 100,000 photos with 50,000 sidecars on a NAS at about 20 s of listing and 2 minutes of sidecars, EXIF at 35 ms a photo the long part.
- **Full text** over file names, keywords, title, description, notes, camera and lens, by darktable's text-search rule. Vectors for semantic search, similar photos and faces live only in the cache, decided with the AI work.
- **Its engine is chosen by spike L2,** between plain Rust collections in memory with a snapshot file and `tantivy` (MIT) for full text, the likely choice, and Turso Database; `rusqlite` with SQLite compiled in only if both fail, and then only with an exception to [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md). L2 measures opening, a smart album of three rules, a full-text search, a rating set on 500 photos, a rebuild from cached sidecars and memory, at 100,000 and 500,000 synthetic photos, against a query under 50 ms and an open under 1 s at 100,000; plain collections cost about 1 kB a photo. Its result is recorded in this record.

## Options considered

- A database as the truth, or on shared storage: against [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md), and broken by sync services.
- Turso preferred before it is measured.
- SQLite as the default: C compiled in.

## Consequences

- Nothing outside layer 3 knows the cache's engine, so L2's choice changes one module.
- A cache of another build's schema is rebuilt, never migrated.
