# 0023. rawler, its camera data fixed upstream

## Context

Which raw decoder does Apertrust use, whose camera data does it trust for crops and levels, and what size is the developed image? darktable decodes with rawspeed, a C++ library ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)), and its camera database differs from other decoders' on crops, black and white levels. darktable is the reference for processing; it is not the reference for camera data.

## Decision

**rawler decodes raws**, in pure Rust, wrapped in `apertrust-raw`'s own types so its pre-1.0 API stays behind one crate. On the 40 raw.pixls.us samples it decodes more than rawspeed does, Canon CR3 included, with identical raw values wherever both decode a mosaic, apart from the known differences below.

**A corpus test pins its output** (`tools/fetch-raw-corpus.sh`, then `cargo test --release -p apertrust-raw -- --ignored` against `tests/corpus/expected.txt`), so updating rawler is deliberate and a changed result is blessed and reviewed (`APERTRUST_BLESS=1`).

**rawler's camera data is trusted, its errors fixed upstream, nothing patched locally.** A difference counts as a rawler bug only when the evidence shows it, never because darktable differs: what rawler's code intends, what the manufacturer's metadata says (read independently with exiftool), what the photosites show (the histogram's floor and clip, whether an edge strip holds image data), and what upstream's history and issues say. A bug goes upstream as a fix; a deliberate choice whose reason is unknown goes upstream as a question with the evidence.

**The developed image:** rawprepare keeps the active area, every valid photosite, so demosaic reads real neighbours at the edges; the developed image is the manufacturer's declared size, rawler's default crop (a DNG's DefaultCrop, the camera's JPEG size), or the active area for a raw that declares none. The crop to the declared size comes after demosaic, through the regions ([0012](0012-planning-regions-scale-and-cache-points.md)).

**Black levels are the stored ones:** a level from the raw or rawler's camera data, and a measurement only where there is none, rawler's own order, whose fallback is the mean of every masked photosite. `apertrust-raw`'s `RawImage::masked_black` reproduces darktable's measurement (rawspeed's `calculateBlackAreas`, with its single-column bug) only to explain darktable's levels, in `apertrust-cli info` and `apertrust-golden raw-defaults`.

**Known decoder differences**, each to be fixed upstream:

- Sony's lossy ARWs: both decoders dither Sony's tone curve with the same table and generator, but rawler seeds each row's generator from 16 bits where rawspeed uses 24. With rawler seeded from 24 bits the samples match.
- The Nikon Z 9's white point is 15520, copied from older bodies; the Z 8, the same sensor, and later bodies have 15892.
- The Samsung NX1's black level is 0, where the data floors near 128.

**The comparisons measure processing, not camera data.** The whole-pipeline comparison and the raw-stage cases write Apertrust's crop and levels into darktable's history ([0062](0062-the-whole-pipeline-comparison.md)); `raw-defaults` tracks each decoder difference with its verdict.

## Options considered

- rawspeed through FFI, or LibRaw: C++ compiled in, and rawspeed does not build with MSVC.
- darktable's camera data and per-format rules over rawler's pixels: a second database to maintain, with darktable's quirks.
- A local table of corrections: drifts from upstream.
- darktable's crop: a few pixels of rawspeed's choosing rather than the manufacturer's.
- Measuring black wherever there are masked areas: moves cameras with stored levels away from both the camera and darktable.
- Dithering again after decoding, or turning dithering off: rawler already dithers; off, the lossy ARWs would differ from darktable on most photosites.

## Consequences

- The cameras darktable measures differ from darktable by a few units in the deepest shadows (the 40D: 1023 against 1030 and 1026).
- Sony's two lossy ARW samples in the corpus fail the whole-pipeline comparison until rawler's dither-seed fix is released.
- Risks: one main maintainer and a pre-1.0 API, mitigated by the wrapper and the corpus test.
