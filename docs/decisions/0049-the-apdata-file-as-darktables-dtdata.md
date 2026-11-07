# 0049. The .apdata file as darktable's .dtdata

## Context

Some of what an edit depends on is binary and cannot be recomputed: painted brush masks, refined AI masks, AI erase results. It does not belong in an XMP text file. darktable's development branch, after the reference release, added a file of the same kind for its AI object masks, `.dtdata` (`dev-doc/dtdata_sidecar.md`), and the first pass mirrors darktable ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Decision

**`IMG_2041.ARW.apdata` is darktable's `.dtdata`.** It is optional: a photo has one only once an edit saves binary or raster data; until then, and for most photos ever, only its `.apxmp` exists ([0048](0048-the-apxmp-format.md)).

- **The format:** a flat zip with no manifest, entries stored, not deflated (they are compressed already): an entry `version` holding `1`, then one PNG per entry named by its kind and the SHA-1 of its bytes, `mask-<sha1>.png` (8-bit grey), `depth-<sha1>.png` (16-bit grey), `brush-<sha1>.png` (8-bit grey), `patch-<sha1>.png` (pixels), new kinds as darktable adds them or Apertrust needs one. Written with `zip` and `png`, pure Rust.
- **The reference** lives in the `.apxmp`, as darktable's `dt_dtdata_ref_t`: the entry's name, kind, origin (made by a person, or regenerable by a model), size, bit depth and producer, and no path. The entry is looked up in the `.apdata` beside that `.apxmp`; its SHA-1 is checked against its name on read, and a missing or damaged entry shows that mask as missing while the rest of the edit opens.
- **Entries never change.** A changed mask is a new entry; each history step, virtual copy and snapshot names the entries it uses, so undo and snapshots find their masks. Readers ignore entries they do not know, and writers keep them, so an older build never drops what a newer one wrote.
- **Writes:** the whole zip rewritten through the safe write ([0053](0053-finding-folders-and-photos.md)); a zip that cannot be read to its end is never rewritten, swept or merged from.
- **Cleanup is darktable's sweep:** an entry goes only when no saved step, virtual copy or snapshot names it, decided once the history is final (the photo left, Compress History), never on reset or undo, and only for kinds a built-in producer registers; the file goes once it is empty.
- **It follows the `.apxmp` everywhere** ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md)): where it lives, and through Move, Rename, Copy, Delete, import, Import library… and Export as Project…; pasting an edit copies the entries its references need, as darktable's `dt_dtdata_merge`.
- **Two copies combine** by taking every entry either holds, which loses nothing, since entries are named by their bytes ([0052](0052-comparing-two-copies-of-a-sidecar.md)).
- **Planned:** with the first raster mask, in Phase 4.

## Options considered

- A format of Apertrust's own (a header, a chunk table, compressed tiles): no other tool reads it, and darktable's masks would need a converter.
- A database, or a folder of loose files per photo: neither travels as one file.

## Consequences

- The library's file operations carry `.apdata` with every `.apxmp` they move, copy or delete.
- darktable's `.dtdata` masks, when its edits are imported ([0051](0051-importing-darktables-edits.md)), need no conversion of the file itself.
