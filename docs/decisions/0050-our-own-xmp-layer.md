# 0050. Our own XMP layer

## Context

Who reads and writes XMP? The `.apxmp` is XMP ([0048](0048-the-apxmp-format.md)), the library reads ratings, labels and keywords from darktable's `.xmp` files, and both must follow the XMP standard, as darktable does through exiv2. No pure-Rust crate both reads and writes XMP trustworthily, and C or C++ needs a strong reason ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).

## Decision

**An XMP layer of Apertrust's own on quick-xml, its target conformance to ISO 16684-1:2019 (which revises ISO 16684-1:2012, whose text Adobe publishes free as the XMP Specification Part 1),** XMP Part 1: data model, serialization and core properties, held by its fixtures: `apertrust_library::xmp`, with no dependency of its own beyond quick-xml.

- **The tree:** `Xmp` (its toolkit, `about`, declared prefixes and properties in order); `Property`; `Node` with `Value` simple, resource, struct, or a Bag, Seq or Alt array; qualifiers, `xml:lang` first.
- **Reading** (`Xmp::parse`, `parse_bytes`): any valid packet, with or without the xpacket wrapper, a BOM or UTF-16, a bare `rdf:RDF`; merged descriptions; attribute and element forms; `parseType="Resource"`; nested descriptions; `rdf:resource`; `rdf:value` qualifiers; CDATA. An error names its line; RDF beyond XMP's subset is refused.
- **Writing:** `to_xml`, a canonical compact form with no wrapper for sidecars, so the same tree gives the same bytes; `to_packet`, with the wrapper, for embedding in files.
- **Round trips** keep every unknown namespace, property and prefix in order; `get`, `set` and `remove` change one property.
- **The standard schemas and darktable's namespace** are read as any other; the layer stays namespace-generic.
- **Held by fixtures** in `crates/library/tests/fixtures/xmp`: darktable's `.xmp` files and hand-written forms, and a version 2 sidecar with a history, added by the library's step 8.2 ([0078](0078-undo-redo-and-the-edits-history.md), planned); ExifTool and exiv2 read what it writes without error. The `.apxmp` text layer goes through it.

**EXIF** is read by `apertrust-raw` from rawler's metadata (`read_exif_file`, `read_exif`), never through this layer. EXIF and XMP written into exported files come with the export's metadata, through a pure-Rust writer chosen then (planned).

## Options considered

- exiftool-rs: it runs ExifTool's Perl through an interpreter and compiles in PCRE2 and zlib, C.
- `xmp_toolkit` and rexiv2: C++.
- xmpkit: too young, one maintainer, an unproven parser.
- `xmp_writer`: writes only.
- RDF/XML crates: no XMP order, languages or canonical form.
- exiv2: C++, an exception to the rule that nothing needs.

## Consequences

- Every XMP the project reads or writes goes through one tree; no hand-written serialiser remains.
- New schemas need no code in the layer, only in the reader that uses them.
