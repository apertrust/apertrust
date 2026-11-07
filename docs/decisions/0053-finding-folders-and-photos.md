# 0053. Finding folders and photos

## Context

Photos live on internal disks, external drives, network shares and cloud folders. Drives change letters and mount points, shares are slow or asleep, cloud files may not be on the disk at all, and people rename files outside Apertrust. How does Apertrust find folders and photos again, read them quickly, and write without damaging them?

## Decision

**Roots** are folders Apertrust tracks as a unit, each carrying its id at its top: a library or project in its file, a plain folder in `.apertrust/root.toml`, written once as it joins a library. Shared files name a photo by its root's id and a path relative to the root, in Unicode NFC with `/`. A root is found by its path relative to the library, then by the same name under each mounted volume with its id file checked, then by Locate…, once per root per machine; the answer goes into a per-machine root map. Never volume UUIDs, drive letters, inodes or absolute paths.

**The fingerprint** is the file's size plus BLAKE3 over its first and last 64 KiB (`blake3`, pure Rust features only), in `apertrust:fingerprint` and the cache. It re-pairs a sidecar with a raw renamed outside Apertrust, recognises moved photos and duplicates on import, and keys thumbnails. It is computed only when a photo is opened or thumbnailed, when its sidecar is first written, and for a lost sidecar's few candidates; never for a cloud file not on the disk. A whole-file hash would cost 0.36 s for a 30 MB raw over a typical share.

**Names:** a sidecar matches its photo by exact bytes, else in NFC with case folding when one photo matches, the fingerprint confirming. Apertrust never renames the person's files; the names it makes pass Windows' and OneDrive's rules on every system. A rename outside Apertrust is re-paired by size and fingerprint, the activity saying so; Move, Rename and Copy inside Apertrust carry sidecars, `.apdata` and attached files.

**Listing:** one folder at a time on a worker thread, cancellable, its raws by `apertrust_raw::extensions()` with their sidecars, in chunks. Bulk listing (`getattrlistbulk`, `FindFirstFileExW` with a large fetch, `statx`) is planned, since a call per file takes 21.6 s for 2,825 files over SMB on macOS against 0.3 s in bulk; spike L1 settles whether it needs an FFI crate, `apertrust-fs` ([0065](0065-names-and-crates.md)). Sidecars are read 16 at a time and compared by name, size and time, then by content hash. The open folder is listed as it opens, when the window comes to the front, and every 30 s on network and cloud roots; watchers on local disks are hints; Check for Changes runs by hand. Nothing reads a raw to detect a change.

**Writing** (`apertrust_base::fs`): only sidecars, a library's files and a root's id file are written into the person's folders, never lock files, caches or indexes. A file is replaced whole through a hidden temporary beside it, `.~NAME.XXXXXX.tmp`, written, flushed, synced and renamed; nothing is written when the file already holds the bytes; on an error nothing is left behind and the old file is as it was. A rename Windows refuses (sharing violation, access denied) is retried for about a second, ten times 100 ms. A listing removes leftover temporaries more than a day old, told by their name alone. Writes run on the library's threads, never the interface's or the engine's.

**A NAS:** each root's calls run on a queue of their own, "Waiting for Main-Storage…" after 2 s, "Not responding" after 30 s, retried every 2 minutes, so the interface never waits for a share.

**Offline drives:** photos keep their thumbnails with an offline mark; ratings, labels, keywords and metadata still change, held in the library's `.apertrust/pending/` until the drive returns; edits need a local copy, kept in the cache by fingerprint.

**Cloud folders:** a file not on the disk is never read in the background, macOS's materialisation turned off on the library's threads; placeholders are told by Windows' recall attributes and macOS's `SF_DATALESS`, shown with a cloud mark; Make Available Offline downloads one.

## Options considered

- Volume UUIDs, drive letters, inodes or absolute paths with a prefix rewritten: they break when a drive moves, as darktable's skulls show.
- No fingerprint: renamed raws lose their edits. A whole-file hash: too slow on a share.
- A call per file on a macOS share: two orders of magnitude slower than bulk listing.
- Lock files: they fail on shares and litter the person's folders.

## Consequences

- `apertrust-library`'s listing, index and sidecar writes follow these rules; the export's disk writes share the safe write.
- Planned: bulk listing, the per-root queues, offline and cloud handling.
