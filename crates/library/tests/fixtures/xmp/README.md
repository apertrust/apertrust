# XMP fixtures

Read by `tests/xmp_fixtures.rs` and the unit tests of `src/xmp`.

- `darktable-*.xmp`: darktable's own sidecars, copied unchanged from its integration tests
  (`src/tests/integration/0001-exposure`, `0011-gdnd-90`, `0095-highlight-opposed`,
  `0176-color-harmonizer`), GPL-3.0-or-later like the rest of the repository. They are
  written by Exiv2's compact format. The folder is darktable's submodule
  `src/tests/integration`, at commit `aaf83597072f865417e0f2d2fe1c90d039b3d142`, the one
  darktable pins.
- `form-*.xmp`: hand-written packets that say the same thing in the forms RDF/XML allows,
  all of which the reader must give one tree: attributes and nested descriptions
  (`form-attributes`), elements and `rdf:parseType="Resource"` (`form-elements`), a bare
  `rdf:RDF` of several descriptions with struct and array items as attributes
  (`form-bare-rdf`), and an `xpacket` with a byte order mark, comments, CDATA, character
  references and padding (`form-packet`).
