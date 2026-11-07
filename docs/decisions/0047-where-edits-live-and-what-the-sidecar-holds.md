# 0047. Where edits live, and what the sidecar holds

## Context

Edits are sidecars ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)). Beside the photo is the natural place, but a read-only folder, a share others own or a library that wants its photos' folders clean needs another. And the sidecar must hold more than today's module values: history that survives a restart, virtual copies, snapshots, and what two machines need to compare copies ([0052](0052-comparing-two-copies-of-a-sidecar.md)).

## Decision

**Edits live beside the photos by default, or per library in the library's own folder, mirrored by path; one place per root, named by the root's id file.**

- Beside: `DSCF0001.RAF.apxmp`. In a library: `Family/.apertrust/edits/<root id>/<path>.apxmp`, the same format byte for byte.
- `.apertrust/root.toml` at a root's top names beside, or one library by id, name and a path hint. Every library holding the root reads and writes there, folder mode too: it looks in the folder and its parents, one look each, says "Edits for these photos are kept in the library Family" and uses them; if that library cannot be found it opens the photos view-only with Locate…, writing nothing beside them.
- The library's setting covers photos copied in and folders added without sidecars; a folder added with sidecars keeps them. A read-only root's edits are always its library's.
- **A user data folder never decides where edits go** ([0043](0043-settings-and-apertrusts-folders.md)).

**`.apxmp` version 2** (planned; version 1 is [0048](0048-the-apxmp-format.md)'s and stays read, as one virtual copy with one step):

- the photo's identity: `apertrust:photo`, `apertrust:fingerprint` ([0053](0053-finding-folders-and-photos.md)) and the build that wrote it;
- `apertrust:copies`: each virtual copy a random id of 8 hex digits, a number (darktable's version), a name, its history, `apertrust:end` (darktable's `history_end`), its snapshots and its clock vector;
- its groups.

**History is a log of steps,** and the session's Undo and History are this log from their first build, so reopening a photo brings Undo back and a crash loses at most the gesture in flight.

- A step is one gesture (a change ending with `done`): an id, a clock, each module it touched with its values after it (`off` for a reset that turns a switched module off, [0032](0032-modules-off-until-first-touched.md)), the ratings, reject, labels, keywords and metadata it set, and a batch's id.
- The edit is each module's values at its last step up to the end, else its defaults. Undo and Redo move the end; a change after Undo drops the steps after it.
- Compress History keeps each module's and field's last entry and the clocks it covered; Discard History is a step. The log alone is stored, never the state beside it: about 100 to 300 bytes a step.

**Virtual copies:** New Virtual Copy (darktable's duplicate) and New Virtual Copy from Original; a deletion leaves a tombstone; a copy is named `photo#id`.

**Snapshots** are saved per virtual copy, each its edit whole as one compressed step, so Compress cannot break one; Restore Snapshot is a step.

**Clocks:** hybrid logical clocks (UTC milliseconds, a counter, the machine id) on every step, snapshot, tombstone and library-file entry, and a clock vector per virtual copy. The machine id is a hash of the system's machine id and the user, never kept in the user data folder; machine names live only in a library's `.apertrust/machines/`.

**`.apdata`** ([0049](0049-the-apdata-file-as-darktables-dtdata.md)) lives wherever its `.apxmp` lives and moves, copies and imports with it.

## Options considered

- A central folder of edits per machine: edits no longer travel with the photos.
- A database as the edits' truth: against [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md).
- History in memory only: lost on a crash, against principle 3.
- The edit's state stored beside its log: two truths to keep equal.
- A snapshot as a point in the log, as darktable's: Compress would break it.
- darktable's numbered sidecars for duplicates: two machines making copies offline clash on the numbers.

## Consequences

- The engine's session keeps history as the log; the library reads the sidecar through its text layer ([0054](0054-four-layers-for-the-librarys-code.md)).
- Open: how labels and the reject are encoded in a step, settled when they are built.
