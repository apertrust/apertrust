# darktable's ratings, labels and metadata

Read by `tests/darktable.rs`: small sidecars in the form darktable 5.6.2 writes
(`_exif_xmp_read_data`, `src/common/exif.cc`, Exiv2's compact format), each holding its
rating and colour labels as darktable writes them and an empty history, hand-made from the
integration tests' sidecars in `tests/fixtures/xmp`.

- `rating-3-labels.xmp`: three stars, the red and blue labels.
- `rejected.xmp`: `xmp:Rating` −1, a reject, as darktable writes one.
- `label-and-colorlabels.xmp`: `xmp:Label` beside `darktable:colorlabels`, as another
  application's label beside darktable's: the label wins, as darktable reads it.
- `labels-alone.xmp`: no stars, the purple and yellow labels in the order set.
- `fields.xmp`: two stars and darktable's metadata fields as darktable 5.6.2 writes them
  (`_set_xmp_dt_metadata`, Exiv2's compact format): the title, a description of two lines
  and the rights as language alternatives, the creator in an `rdf:Seq`, the publisher in
  an `rdf:Bag`, its notes in `acdsee:notes` and its version name in
  `darktable:version_name`, hand-made from `darktable-highlight-opposed.xmp` in
  `tests/fixtures/xmp`, which darktable wrote.
