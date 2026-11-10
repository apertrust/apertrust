# 0052. Comparing two copies of a sidecar

## Context

People edit on more than one computer, through a share, a sync service or a drive carried between them. Two copies of one sidecar will meet: a conflict copy, a sidecar changed outside Apertrust, a file found changed just before a write. Locks do not work across NFS and SMB and do not exist in cloud folders. How are two copies combined without losing anyone's work?

## Decision

**Two copies are compared virtual copy by virtual copy, by id, through their clock vectors** ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md)):

- the same vector: nothing to do;
- one at or below the other for every machine: that copy is only older, and the newer replaces it;
- neither, and both changed the edit: the copy whose last step is later stays, and the other becomes a new virtual copy with its own history, named after its machine and time ("From MacBook, 5 Nov 14:02"; in folder mode "From another computer"), the activity saying so.

**Ratings, the reject, labels, keywords and metadata combine:** the two logs join step by step in clock order, the later change winning each field, label and keyword, so culling on one computer and editing on another never splits a photo. A virtual copy only one side has is kept; a deletion wins only over steps older than it. A module-by-module merge of edits comes later, as a setting.

**When copies are compared:**

- before every write: the writer reads the file again if it changed since this machine last read it, and writes only if the bytes differ;
- when a sidecar changes outside Apertrust: the active photo takes the newer edit, a module being dragged stays the person's, and a diverged photo stays on screen with the other edit arriving as a virtual copy;
- conflict copies: any file ending in `.apxmp` that is not a photo's name plus `.apxmp` and whose `apertrust:photo` and fingerprint match a photo there, compared, kept 30 days in the user data folder's `conflict-copies/` and removed ([0076](0076-the-cache-kinds-limits-and-control.md)); its content decides, not its name;
- a server root's sidecar, whenever this machine and the server exchange it, the server being one more copy ([0071](0071-a-self-hosted-server-with-sync.md), planned);
- later: silent winners, each machine keeping in its user data folder the last copy of each sidecar it wrote ([0076](0076-the-cache-kinds-limits-and-control.md)), and iCloud's hidden versions.

**Convergence:** comparing the same copies gives the same bytes on every machine, so after one round nothing is written again.

**A library's own files** combine entry by entry, the later clock winning each, so photos added to an album on two machines are all kept. `.apdata` copies combine by taking every entry ([0049](0049-the-apdata-file-as-darktables-dtdata.md)).

**Older builds are strictly view-only** for what a newer one wrote, metadata included ([0048](0048-the-apxmp-format.md)): nothing in such a sidecar is changed or compared, and its conflict copies are left for a newer build. A newer library or project file opens view-only; an album or smart album file holding what this build does not know is shown and never written, a smart album with a rule it cannot read showing no photos and saying so.

**No lock decides anything.** A presence note may later warn that a library is open on another computer; it blocks nothing.

## Options considered

- A module-by-module merge first: more work than the first build needs; kept as a later setting.
- A CRDT library such as Automerge: a binary document, against a readable XMP sidecar.
- darktable's keep-one-side-whole by modified time: one machine's work is lost.
- Locks: they fail on NFS and SMB and do not exist in cloud folders.

## Consequences

- Every sidecar write in the library reads before it writes.
- A test harness of simulated machines and syncs holds convergence.
