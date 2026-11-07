# 0048. The .apxmp format

## Context

What does an edit file look like, and how is it read so that a photo is never lost to a file it cannot fully understand, and never damaged by a build older than the one that wrote it? Edits are sidecars beside the photo ([0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md)), and XMP is what other photo tools read.

## Decision

**XMP beside the photo,** `IMG.ARW.apxmp`: one `rdf:Description` with `apertrust:version`, the build that wrote it in `x:xmptk`, and its entries: each module's values with the module's version and its marks, and the operation parameters the edit sets only where no module owns them ([0028](0028-modules-bound-to-operations.md)). Values left out keep their defaults for the photo.

- **Numbers** are written in their shortest exact form and **choices by name**, so a reordered list changes no edit.
- **The namespaces** are on the project's domain: `https://apertrust.org/ns/xmp/` for Apertrust's own properties (prefix `apertrust`), `https://apertrust.org/ns/xmp/control/` for module controls (`control`) and `https://apertrust.org/ns/xmp/param/` for operation parameters (`param`). A URI is an identifier, so nothing is hosted there; the schema's version is `apertrust:version`, not part of the URI. Nothing else is read: a file in another namespace is not an Apertrust edit (`SidecarError::NotAnEdit`).
- **The current version is 1;** version 0 still opens. Version 2, with history and virtual copies, is planned ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md)).

**Reading never loses the photo.** An unreadable value or a number out of range is left out or clamped with a warning, and the photo opens. A file that is not an Apertrust edit is an error, never a silent default.

**A save never drops what the edit holds.** Whatever this build would leave out makes the photo **view-only**: its controls disabled, every change refused, the file never written, and the photo's caption saying which version wrote it. View-only follows from what the file holds, never from comparing build versions:

- a newer file version, a newer module version, or a module this build neither knows nor has retired (`apertrust_modules::RETIRED`);
- at a module's own version, a control it lacks, a mark on one, or a value it cannot read or keep;
- an operation it lacks, a schema version above its own, or at its own a parameter it lacks;
- an entry repeated, or any element or attribute it does not write.

Left out with a warning, the photo still editable: a retired module, what an older module or schema version held that this one lacks, and a parameter a module owns.

**Versions rise for removals and changes, never for additions:** a module's, an operation's schema and the file's version rise when what an older build wrote would read differently or not at all (removed or renamed, a range narrowed, a default or meaning changed). An addition whose default keeps old edits' look raises nothing. One addition does raise a module's version: allowing an option that older builds hold back, so that those builds open an edit holding it view-only rather than leave the value out and drop it at their next save. A committed record of each module's stored controls and of the parameters an edit can store holds this in tests, and committed sidecars must read with no warning and write back byte for byte.

**Writing** has one fixed layout, so a file read and written back is the same bytes; it goes through the XMP layer ([0050](0050-our-own-xmp-layer.md)) and the safe write: a hidden temporary beside the file, flushed, synced and renamed, nothing written when the file already holds the bytes ([0053](0053-finding-folders-and-photos.md)).

**Where the code is:** the text layer, `apertrust_library::sidecar` (`Document`, its version, build and entries as strings in the file's order, `Newer`, `ViewOnly`, `path_for`, `write_xml`), knows no module or number's meaning; the engine's `sidecar` turns entries into an `Edit` and back with the module definitions.

## Options considered

- darktable's layout, an `rdf:li` per history item with hex-encoded parameter structs: no other tool can read it.
- JSON or TOML: not XMP, which ExifTool and other tools read.
- Parameters as child elements: longer for nothing. Choices by index: a reordered list would change edits.
- Keeping what a build does not know on save: an older build would edit a photo it renders unlike the build that wrote it.
- Raising a version for every addition: versions on two branches do not order, and each raise locks older builds out of edits that do not use the addition.
- Namespaces under a hosting service's address: the project would outlive it.

## Consequences

- Every module and operation version change comes with its migration and an update of the committed record.
- An export from a saved edit is byte-identical to one with the same `--set` values.
