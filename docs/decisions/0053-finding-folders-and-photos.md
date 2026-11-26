# 0053. Finding folders and photos

## Context

Photos live on internal disks, external drives, network shares and cloud folders. Drives change letters and mount points, shares are slow or asleep, cloud files may not be on the disk at all, and people rename files outside Apertrust. How does Apertrust find folders and photos again, read them quickly, and write without damaging them?

## Decision

**Folder records** ([0101](0101-folders-first-libraries-optional.md), planned): a folder this computer must find again (in a Library, kept offline, keeping its edits here, or a saved Library's home folder) has a record in the user data folder, `folders/<id>.toml`, holding its last path, its volume's id and its path below the mount. Shared files name a photo by a path relative to the folder that holds them, with `/`, its name's bytes kept as read, and its fingerprint, matched by exact bytes first, then in NFC where only one file matches, as a sidecar's name is (below), so two names that differ only in their Unicode form stay apart: a folder's albums relative to that folder, a Library's by its own folder id and a relative path. A folder is found by its last path, then by its volume's id and the path below the mount (macOS's volume UUID, Linux's filesystem UUID, Windows' volume GUID, a share's server and share name, from the platform's volume list), the photos' fingerprints confirming, then by Locate…, once per folder per computer, accepted once enough of its photos match by fingerprint; the answer goes into the record. **A volume id is a hint, never the identity;** a clone with the same id at two places asks which is the folder. Never drive letters, inodes or absolute paths as the identity, and nothing written into the folder to mark it. A plain recent keeps no record: a moved mount shows it as Not found, with Locate….

**Away, never deleted:** a volume or share not present means a folder is away; a file gone from a present volume means it is missing, re-paired by fingerprint when it comes back. No index row, album entry or pending change is dropped by a scan, only by the person.

**A server folder** (later, if asked, [0071](0071-a-self-hosted-server-with-sync.md)) would be a folder record found by its server's URL, an account and the folder's path on the server instead of a local path, its calls through a storage interface ([0054](0054-four-layers-for-the-librarys-code.md)), on its own queue as a share's do.

**The fingerprint** is the file's size plus BLAKE3 over its first and last 64 KiB (`blake3`, pure Rust features only), in `apertrust:fingerprint` and the cache. It re-pairs a sidecar with a raw renamed outside Apertrust, recognises moved photos and duplicates on import, and keys thumbnails. It is computed only when a photo is opened or thumbnailed, when its sidecar is first written, for a lost sidecar's few candidates and for Import's candidates of the same size and camera name ([0085](0085-import-in-folder-mode.md)); never for a cloud file not on the disk. A whole-file hash would cost 0.36 s for a 30 MB raw over a typical share.

**Names:** a sidecar matches its photo by exact bytes, else in NFC with case folding when one photo matches, the fingerprint confirming. Apertrust never renames the person's files; the names it makes pass Windows' and OneDrive's rules on every system. A rename outside Apertrust is re-paired by size and fingerprint, the activity saying so, wherever the orphan sidecar and the photo are in one indexed scope, the open folder with its subfolders or a Library; across scopes, Find Photos… on the orphans. Move to…, Rename and Copy to… inside Apertrust carry sidecars, `.apdata` and attached files, across volumes by [0101](0101-folders-first-libraries-optional.md)'s checked copy.

**Listing:** one folder at a time on a worker thread, cancellable, its raws by `apertrust_raw::extensions()` with their sidecars, and, planned, the other images Apertrust opens ([0111](0111-images-other-than-raws-opened-and-edited-as-darktable-does.md)), in chunks. Bulk listing (`getattrlistbulk`, `FindFirstFileExW` with a large fetch, `statx`) is planned, since a call per file takes 21.6 s for 2,825 files over SMB on macOS against 0.3 s in bulk; spike L1 settles whether it needs an FFI crate, `apertrust-fs` ([0065](0065-names-and-crates.md)). Sidecars are read 16 at a time and compared by name, size and time, then by content hash. The open folder is listed as it opens, when the window comes to the front, and every 30 s on network and cloud folders; watchers on local disks are hints; Check for Changes runs by hand. Nothing reads a raw to detect a change.

**Writing** (`apertrust_base::fs`): only sidecars, a folder's `.apertrust/` (its albums, and a Library saved there by choice) and the copies Import, Move to… and Copy to… make ([0085](0085-import-in-folder-mode.md), [0101](0101-folders-first-libraries-optional.md)) are written into the person's folders, never lock files, caches, indexes or a mark of the folder. A file is replaced whole through a hidden temporary beside it, `.~NAME.XXXXXX.tmp`, written, flushed, synced and renamed; nothing is written when the file already holds the bytes; on an error nothing is left behind and the old file is as it was. A rename Windows refuses (sharing violation, access denied) is retried for about a second, ten times 100 ms. A listing removes leftover temporaries more than a day old, told by their name alone. Writes run on the library's threads, never the interface's or the engine's.

**A NAS:** each folder tree's calls run on a queue of their own, "Waiting for Main-Storage…" after 2 s, "Not responding" after 30 s, retried every 2 minutes, so the interface never waits for a share.

**Offline drives:** a folder kept offline or in a Library keeps one thumbnail per photo, with an away mark; ratings, labels, keywords and metadata still change, held in its folder record's `pending/` in the user data folder until the drive returns; editing waits for the drive, local copies of raws being later, if asked ([0101](0101-folders-first-libraries-optional.md)).

**Cloud folders:** a file not on the disk is never read in the background, macOS's materialisation turned off on the library's threads; placeholders are told by Windows' recall attributes and macOS's `SF_DATALESS`, shown with a cloud mark; Make Available Offline downloads one.

## Options considered

- Drive letters, inodes or absolute paths with a prefix rewritten as the identity: they break when a drive moves, as darktable's skulls show. Volume ids as the identity: a clone or a reformatted drive would be taken for the folder.
- Roots marked by an id file at their top and found through a per-machine root map: a file written into every folder added, and a library to hold the map.
- No fingerprint: renamed raws lose their edits. A whole-file hash: too slow on a share.
- A call per file on a macOS share: two orders of magnitude slower than bulk listing.
- Lock files: they fail on shares and litter the person's folders.

## Consequences

- `apertrust-library`'s listing, index and sidecar writes follow these rules; the export's disk writes share the safe write.
- Planned: bulk listing, the per-folder queues, folder records, offline and cloud handling; server folders later, if asked.
