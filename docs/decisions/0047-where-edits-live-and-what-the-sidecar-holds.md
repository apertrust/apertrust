# 0047. Where edits live, and what the sidecar holds

## Context

Edits are sidecars ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)). Beside the photo is the natural place, but a read-only folder, a share others own or a library that wants its photos' folders clean needs another. And the sidecar must hold more than today's module values: history that survives a restart, virtual copies, snapshots, and what two machines need to compare copies ([0052](0052-comparing-two-copies-of-a-sidecar.md)).

## Decision

**Edits live beside the photos by default, or per library in the library's own folder, mirrored by path; one place per root, named by the root's id file.**

- Beside: `DSCF0001.RAF.apxmp`. In a library: `Family/.apertrust/edits/<root id>/<path>.apxmp`, the same format byte for byte.
- `.apertrust/root.toml` at a root's top names beside, or one library by id, name and a path hint. Every library holding the root reads and writes there, folder mode too: it looks in the folder and its parents, one look each, says "Edits for these photos are kept in the library Family" and uses them; if that library cannot be found it opens the photos view-only with Locate…, writing nothing beside them.
- The library's setting covers photos copied in and folders added without sidecars; a folder added with sidecars keeps them. A read-only root's edits are always its library's.
- **A user data folder never decides where edits go** ([0043](0043-settings-and-apertrusts-folders.md)).

**`.apxmp` version 2** (built, the engine writing it with the edit's history; version 1 is [0048](0048-the-apxmp-format.md)'s and stays read, as one virtual copy with a step of each module):

- the photo's identity: `apertrust:photo`, `apertrust:fingerprint` ([0053](0053-finding-folders-and-photos.md)) and the build that wrote it;
- `apertrust:copies`: each virtual copy a random id of 8 hex digits, a number (darktable's version), a name, its history, `apertrust:end` (darktable's `history_end`), its snapshots and its clock vector;
- its groups.

**History is a log of steps,** and the session's Undo and History are this log, kept in the session's memory in their first build and saved in the sidecar as soon as version 2 lands ([0078](0078-undo-redo-and-the-edits-history.md)), so reopening a photo brings Undo back and a crash loses at most the gesture in flight.

- A step is one module's change, a completed change of several modules a step of each sharing a batch id, a control's quick repeats merged ([0078](0078-undo-redo-and-the-edits-history.md)): an id, a clock, its source and an optional note, the module it touched with its values after it, or Reset's every module (`off` for a reset that turns a switched module off, [0032](0032-modules-off-until-first-touched.md)), the ratings, reject, labels, keywords and metadata it set, and a batch's id ([0078](0078-undo-redo-and-the-edits-history.md)).
- The edit is each module's values at its last step up to the end, else its defaults. Undo and Redo move the end, `apertrust:end` holding the last applied step's id, and take back what the session's undo records hold while the photo is open; a change after Undo drops the steps after it.
- Compress History keeps each module's and field's last entry, and advances the copy's clock vector with a clock of its own, so a copy still holding the folded steps is known to be older; Undo puts the log back while the photo is open. Each move of the end advances the clock vector likewise. Reset is a step; Discard History… clears the log and resets the edit, darktable's discard ([0078](0078-undo-redo-and-the-edits-history.md)). The log alone is stored, never the state beside it: about 720 bytes a step as written, one module's step of two controls set and kept, so 1,000 steps are about 0.7 MB, read within 20 ms and written within 50 ms on an M4's SSD ([history.md](../history.md#in-the-sidecar)).
- The commands, the event and the rest of the design are [0078](0078-undo-redo-and-the-edits-history.md)'s ([history.md](../history.md)).

**Virtual copies:** New Virtual Copy (darktable's duplicate) and New Virtual Copy from Original; a deletion leaves a tombstone; a copy is named `photo#id`.

**Snapshots** are saved per virtual copy, each its edit whole as one compressed step, so Compress cannot break one; Restore Snapshot is a step.

**Clocks:** hybrid logical clocks (UTC milliseconds, a counter, the machine id) on every step, snapshot, tombstone and library-file entry, and a clock vector per virtual copy. The machine id is a hash of the system's machine id and the user, never kept in the user data folder; machine names live only in a library's `.apertrust/machines/`.

- **A machine moves its clock past each clock it reads, within a bound:** another machine's clock read more than 5 minutes past this machine's wall clock (`clock::MAX_DRIFT`, Kerberos's default skew) is ignored, not clamped, by `Clock::observe`. The file is still read and the clock kept as written; this machine's clock does not follow it. So one sidecar dated 9999, or written by a computer whose date or time zone is wrong, never pins every reader's clock, and no other machine's file moves a machine's clocks more than 5 minutes past its wall clock.
- **A machine's own clocks are always followed,** however far past its wall clock: after its wall clock is set back, a step it makes still orders after the steps it made before, so comparing copies machine by machine ([0052](0052-comparing-two-copies-of-a-sidecar.md)) never takes its later step for an earlier one.

**The first virtual copy's id is `00000000`,** the copy a version 1 file reads as, the same on every machine, so two machines changing the same version 1 file compare one copy; a new copy's random id is never `00000000`.

**`.apdata`** ([0049](0049-the-apdata-file-as-darktables-dtdata.md)) lives wherever its `.apxmp` lives and moves, copies and imports with it.

## Options considered

- A central folder of edits per machine: edits no longer travel with the photos.
- A database as the edits' truth: against [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md).
- History in memory only: lost on a crash, against principle 3.
- The edit's state stored beside its log: two truths to keep equal.
- A snapshot as a point in the log, as darktable's: Compress would break it.
- darktable's numbered sidecars for duplicates: two machines making copies offline clash on the numbers.
- No drift bound, a machine following any clock it reads: one sidecar dated 9999 pins the clock of every machine that reads it, its new steps all at 9999 with a rising counter.
- A far-future clock clamped to the bound rather than ignored: the reader's clock would run up to the bound ahead of its wall clock after each such read, its steps showing those times and passing them on to other machines, and still order before the far-future clock, so clamping buys nothing.
- A tighter bound, as a database cluster's half second: computers that set their clocks from the network agree within seconds, but a laptop that has not synced for a while can be minutes off, and its honest clocks would be ignored. A wider one, hours: a time zone set wrongly would move every reader's clocks by hours.
- The bound applied to a machine's own clocks too: after its wall clock is set back, it would ignore its own earlier clocks and order a new step before them.
- A random or hash-derived id for the first copy: two machines converting one version 1 file must give its copy the same id, and a hash of the file differs once either has changed it.

## Consequences

- The engine's session keeps history as the log; the library reads the sidecar through its text layer ([0054](0054-four-layers-for-the-librarys-code.md)).
- A clock ignored still orders where it is: a step a machine makes after reading it orders before it, and a copy holding it counts as later when two copies diverge ([0052](0052-comparing-two-copies-of-a-sidecar.md)) until the wall clocks pass it.
- A machine whose wall clock once ran far ahead keeps making far-future clocks on the photos it edited then, since it follows its own clocks, and other machines ignore those clocks.
- Open: how labels and the reject are encoded in a step, settled when they are built.
