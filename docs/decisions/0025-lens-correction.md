# 0025. Lens correction

## Context

darktable corrects lenses from two sources: the corrections many raws embed (Sony, Fujifilm, DNG opcodes, Olympus and OM System), and lensfun's database. lensfun is a C library that needs GLib ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)). How does Apertrust correct lenses, and how close to darktable's result?

## Decision

**Both of darktable's sources, embedded corrections first.** The module is darktable's lens correction, its method choosing the source ([0029](0029-a-module-is-darktables-page.md)).

**Embedded corrections, as darktable reads and applies them:**

- **The reader** is Apertrust's own in `apertrust-raw`, over rawler's public parsers with no new dependency. Fujifilm's is built: the FujiIFD and CropMode from its maker note. Planned: Sony by `GenericTiffReader` (0x7032, 0x7035 and 0x7037 from the first SubIFD, else IFD0); DNG by `Decoder::ifd` and a bounds-checked port of `dt_dng_opcode_process_opcode_list_3`; Olympus by `orf::parse_makernote` inside `catch_unwind`, since it slices the maker note unchecked. A zero SRATIONAL denominator reads as 0, as Exiv2 reads it. The corpus test pins the values against Exiv2's dump. The values reach the operation through the image context ([0024](0024-the-cameras-data-in-the-image-context.md)).
- **The arithmetic** is darktable's, in one pass: each tap divided by its own vignetting gain after the manual vignette's, in darktable's order, so the four fine-tunes' derivatives are exact and nothing is held; the image scale is not differentiated. If one pass measures slower than two would, that is reported.
- **The bounds** are darktable's: positions clamped to the input region of the untiled output's requirement and taps mirrored at its edges, the same in every tile and pipe ([0016](0016-geometry-for-operations-that-move-pixels.md)).
- **Fujifilm's vignetting table** is ported literally, 16 entries with zeros past `ncin` and the divisor's floor at 1e-4; a test holds that no corpus raw reaches past its last knot.
- **Only manual vignette** is ported with the manual vignette, its gain then a copy.
- **Corrections done:** the operation crate offers a pure port of `_check_corrections_md`; the session computes it when the module's values or the photo change and reports it with the module's state.
- **The module is switched** ([0032](0032-modules-off-until-first-touched.md)): off on a new photo, as darktable's lens correction is, and on with darktable's `reload_defaults` values, every correction, at its first change.
- **Golden cases** take raws from darktable's tests and from the raw.pixls.us corpus, so all four types are compared with darktable.

**lensfun's database** (planned): its lookup and interpolation ported to Rust, its database read directly, its models in a Slang kernel.

## Options considered

- lensfun through FFI: C and GLib compiled in.
- A lens-profile system of Apertrust's own: a database to build and maintain.
- exiftool-rs for the embedded tags: a large dependency that compiles C, names neither Olympus tag and leaves OpcodeList3 a blob.
- Waiting for rawler to expose each format's IFDs and maker notes: a request is drafted; the reader works without it.
- The image's edges for the warp's clamping: differs from darktable in the last columns and rows.
- Two passes, vignetting apart: one more full pass (about 27 ms at 61 MP on the M4) and 1 GB of scratch.
- Fujifilm cases only: three of the four types never checked against darktable.

## Consequences

- Lens correction's embedded method needs the geometry's warp step and its bounds; its spec is `docs/ports/lens.md`.
- The lens's identity, focal length, aperture and distance join the image context with lensfun's method.
