# 0111. Images other than raws, opened and edited as darktable does

## Context

Apertrust opens raws only: `apertrust-raw` decodes them with rawler, the session opens nothing else, and a folder lists its raws alone. darktable also opens JPEG, TIFF, PNG and the other images its loaders read (`src/imageio/imageio.c` at the reference release: `_supported_ldr`, `_supported_hdr` and the magic numbers in `_magic_signatures`), and edits them through the same pipeline with defaults of their own. Raws stay Apertrust's primary target; other images are a required, secondary part of the product.

Three rules constrain how: darktable is the reference while the migration lasts ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), so its handling of these images is ported and golden-tested; no C or C++ is compiled in or shipped unless no Rust option exists ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)), where darktable reads them through libjpeg, libtiff, libpng, libwebp, libjxl, libavif, OpenJPEG, OpenEXR, libheif and GraphicsMagick or ImageMagick, all of which its official build ships; and the pipeline stays GPU-resident and fp32 with no CPU path ([0007](0007-a-software-gpu-instead-of-a-cpu-path.md)).

## Decision

**Apertrust opens and edits images that are not raws as darktable does**, darktable's handling of them ported from its loaders, `colorin.c` and each module's `reload_defaults` at the reference release. "Image that is not a raw", "non-raw" for short, is darktable's image without `DT_IMAGE_RAW`; among them, darktable's loaders mark `DT_IMAGE_HDR` an image of floats and a PNG, JPEG XL, AVIF or HEIF of more than 8 bits, and the rest `DT_IMAGE_LDR`, a 16-bit TIFF among them. Apertrust's decoders mark each as darktable's loader does.

### The formats, in tiers

Each tier is a slice of the plan ([roadmap.md](../roadmap.md#images-other-than-raws)). Every decoder is pure Rust, reads the image of a file of several that darktable's loader reads, a HEIF's or an AVIF's primary image, its profile that image's own, and the first image of the rest, and gives the file's samples, its ICC profile or cICP, its EXIF and its XMP. darktable's alpha is dropped, as its loaders drop it.

| Tier | Formats | Decoder | What it reads, as darktable's loader |
| --- | --- | --- | --- |
| 1 | JPEG | jpeg-decoder or zune-jpeg, both in the tree, chosen by a comparison with darktable's own decode (below); EXIF and XMP through rawler's `formats::jfif`, the reader `apertrust-raw` already uses | 8 bits, grey or YCbCr, baseline and progressive, its ICC profile from its APP2 segments; CMYK and 12-bit refused, as darktable's libjpeg read refuses them |
| 2 | TIFF, PNG | `tiff` and `png`, both in the tree | TIFF: 8 and 16 bits, 16- and 32-bit floats, striped and tiled, a palette expanded (darktable hands tiled and palette files to GraphicsMagick, which gives the same samples), its ICC tag; CIELAB and ICC Lab files of 8 and 16 bits converted to linear Rec2020 as they are unpacked, as darktable's loader converts them (`_read_chunky_8_Lab`, `_read_chunky_16_Lab`), colorin then taking linear Rec2020 as their profile, as darktable's loader reports it; CMYK refused. PNG: 1 to 16 bits, palette and grey expanded, cICP first, then its ICC profile |
| 3 | WebP, JPEG XL, AVIF | `image-webp`; `jxl-oxide`, in the tree through rawler; `rav1d` with its assembly off, with `avif-parse` for the container and libavif's YUV to RGB conversion ported | Each one's ICC profile, AVIF's before its cICP; JPEG XL and AVIF above 8 bits as HDR |
| 4 | OpenEXR, Radiance HDR, PFM | `exr`; a port of darktable's `imageio_rgbe.c`; a reader of Apertrust's own, as the golden harness has | Linear floats, values above 1 kept; EXR's chromaticities and Radiance's primaries as the image's own matrix, darktable's `d65_color_matrix` |
| 5 | JPEG 2000, binary PNM, QOI, GIF, BMP | `hayro-jpeg2000`; ports of darktable's `imageio_pnm.c` and its `qoi.h`; `gif`; a BMP reader of Apertrust's own | JPEG 2000's ICC profile; PNM's 8 and 16 bits; GIF's first frame; BMP's uncompressed and RLE files, the rest refused |

**HEIF and HEIC are a tier of their own, the sixth,** its decoder chosen after a spike that compares the pure-Rust `heic` crate with libheif, darktable's, on a corpus of phones' photos. Rust has no other HEVC decoder, so the tier's choice is open among four:

1. the `heic` crate (0.1.6, AGPL-3.0-only or a commercial licence): pure Rust and the same pixels everywhere, but `deny.toml` would have to allow AGPL-3.0, which GPLv3's section 13 lets a GPLv3 work combine with, and Apertrust would ship an HEVC decoder, with its patent pools;
2. the platform's decoders, ImageIO on macOS and WIC with the HEVC extension on Windows, none on Linux: no exception, but their YCbCr to RGB conversion and chroma upsampling differ by platform, against the same image on every computer, and Linux has none;
3. libheif with libde265, as darktable's build ships them: an exception to decision 5 for C and C++, LGPL, and the same patent question as the first;
4. no HEIF or HEIC at all.

**Left out, decided:** darktable's last fallback, `dt_imageio_open_exotic`, which hands any file its own loaders refuse to GraphicsMagick, then ImageMagick, both C. It reads GIF and BMP, taken in tier 5 by Rust decoders, and a long tail no photographer's workflow needs: XCF, XPM, MNG, JNG, Cineon, DPX, DjVu, FITS, Paint Shop Pro, FBM, FLIF, JBIG2, DICOM, PAM, ASCII PNM, and the CMYK TIFFs that only it converts. A CMYK JPEG darktable refuses outright, and so does Apertrust. Such a file is listed as another file and, opened, refused with its format named. darktable's other names in `_supported_ldr` (`cap`, `cs1`, `bmq`, `qtk`, `rdc`, `pxn`, `gpr`, `cine`) are raws, rawler's to read ([0023](0023-rawler-its-camera-data-fixed-upstream.md)).

**The decoders live in `apertrust-raw`,** a module beside rawler's wrapper (`apertrust_raw::image`, planned), so one crate reads every file a photo can be and its metadata; no new crate. A file is opened by its magic number first, as darktable's `_open_by_magic_number`, so a JPEG named `.heic` opens; a folder lists by extension, the formats of the tiers built.

### In the engine

- **The image context says what the image is:** `ImageContext::kind`, an `ImageKind` of `Raw`, `Ldr` and `Hdr`, darktable's three flags, `camera` `Some` exactly for a raw; and `ImageContext::profile`, the file's own colour description for colorin's defaults (a `FileProfile`: its ICC profile, its cICP, EXIF's colour space, or its own matrix, `None` on a raw). Planned in `apertrust-plugin`; every `defaults(image)` already takes an image with no camera, as the golden cases' inputs are.
- **Uploaded once in its own samples,** 8 or 16 bits, half or full floats, as the mosaic is in its 16 bits, then unpacked by a pass of the engine's own into RGBA f32, each sample over the file's own maximum, which the decoder gives with the samples, as darktable's loaders scale it: 255 or 65535 by its bits, a PNM's from its header (`imageio_pnm.c`), an AVIF's of 10 or 12 bits `(1 << bits) - 1`; floats as they are. The plan starts there, at the image demosaic would have written: no raw stage. The unpack runs in row strips where the image is larger than one binding ([0014](0014-row-strips-for-images-over-one-binding.md)); the viewport's downscale and the full-resolution cache point follow it as they follow demosaic.
- **darktable's v5.0 JPEG order** (`v50_jpg_order`, which `dt_dev_auto_apply_presets` gives an image that is not a raw): the operations darktable places between demosaic and colorin, which expect linear RGB, run after colorin, in that list's order, before colour calibration. Haze removal, orientation, exposure and the tone equalizer are among them today. Each such operation's slot gains its number in that order, between colorin's 28 and 28.5 (`Slot::non_raw`, planned); the planner sorts a non-raw's instances by it; `crates/operations/order.md` gains the column, and `tests/order.rs` holds it to the list ([0022](0022-pipeline-order-and-the-default-pipeline.md)).
- **colorin's input profile, by darktable's `reload_defaults` chain:** the embedded ICC profile (PNG's cICP before it, AVIF's and HEIF's after it); else, for an LDR image, EXIF's colour space (sRGB for 1, Adobe RGB for 2, and for 0xFFFF Adobe RGB where the interoperability index is R03, sRGB where it is R98); else sRGB for an LDR image; else the image's own matrix (EXR, Radiance); else linear Rec709. So an untagged 16-bit PNG is linear Rec709 and an untagged 16-bit TIFF sRGB, as darktable reads them, and PFM, which carries no profile, linear Rec709.
- **colorin's profile maths, through moxcms** ([0026](0026-colour-profiles-and-colour-management.md)), in `apertrust-color`, as darktable's `dt_colorspaces_get_matrix_from_input_profile`:
  - **a matrix-shaper profile,** sRGB and Adobe RGB among them: each channel's tone curve sampled at 65,536 points into a table the kernel interpolates as darktable's `lerp_lut`, with darktable's exponential fit above 1 (`dt_iop_estimate_exp`), then the profile's colorants, D50, to the working space in one matrix;
  - **a LUT-based profile,** where darktable runs lcms2's transform on the CPU: the profile's transform evaluated by moxcms into a 3D table over the encoded input, interpolated tetrahedrally in the kernel, then its connection space to the working space; its tolerance against darktable measured;
  - a profile moxcms cannot read, or of a colour space other than RGB, falls back as darktable's does, to linear Rec709.
- **darktable's defaults for a non-raw,** each operation's `defaults(image)`: no raw stage; exposure at 0 EV, its scene-referred +0.7 EV and black level only on a raw (`dt_image_is_rawprepare_supported`); no tone mapper or colour calibration, whose auto-applied presets are for raws (`FOR_RAW | FOR_MATRIX`), nor white balance or highlight reconstruction, which darktable enables on raws alone; orientation from EXIF; colorout's sRGB. An unedited JPEG runs the unpack, colorin, orientation and colorout.
- **The tools on a non-raw,** each spec saying what darktable's page shows:
  - Raw black/white point and Demosaic are unavailable, drawn disabled, where darktable shows "not applicable" ([0030](0030-controls-as-darktables.md));
  - White balance and Highlight reconstruction run on the image's RGB at darktable's places, as darktable's do on a non-raw, off until touched, highlights with clip highlights alone, by RGB passes of their operations;
  - a tool over an operation every raw runs but a non-raw does not, Exposure, Colour calibration and the tone mapper among them, is switched on a non-raw: off until a control is written, then on at darktable's defaults ([0032](0032-tools-off-until-first-touched.md), [0033](0033-one-tone-mapper-per-edit.md)), its default step absent from a new non-raw's history ([0078](0078-undo-redo-and-the-edits-history.md));
  - Exposure's Mode lists Manual alone, as built ([0088](0088-exposures-page-as-redesigned.md)).
- **Opening in stages** ([0077](0077-a-photo-opens-in-stages.md)): the first picture is the cached render, else the image itself decoded at the Viewer's size; its values come from the file's header and sidecar before the decode; the live frame after it.

### In the library

- **Photos like raws:** a non-raw is a photo in folders, the Browser, one photo and the CLI's `library scan`, its sidecar beside it by its whole name (`photo.jpg.apxmp`), its EXIF and XMP read as a raw's, a JPEG's IPTC-IIM by a reader of its own, planned, since rawler's JPEG parser skips it ([0047](0047-where-edits-live-and-what-the-sidecar-holds.md), [0110](0110-the-metadata-tool-groups-after-exif-and-iptc-metadata-as-values-beside-the-edit.md)).
- **A raw and its JPEG** stay step 17's group by base name, which this decision does not replace. Until step 17, a JPEG with a raw of the same base name beside it is left out of the Browser and the folder's photos, as darktable's collapsed group shows one photo; a JPEG alone is a photo. The JPEG is still opened by every route that names it, File › Open… among them.
- **Its thumbnail is the image itself,** scaled and turned upright, where `library.raw_thumbnails_from` would show a raw's embedded picture, and a render above that level, as darktable's `_init_8` does for a `.jpg`; Apertrust takes a `.jpeg`, a TIFF and a PNG's own picture too, where darktable reads exiv2's small EXIF thumbnail ([library.md](../library.md#thumbnails)).
- **Import** lists and copies non-raws with its slice 20b ([0085](0085-import-in-folder-mode.md)).

### Checked against darktable

- **The decode,** apart from the processing: darktable-cli renders the file with colorin and colorout at linear Rec2020 and Orientation (`flip`) off, which gives its loader's samples as floats in the file's own layout, and Apertrust's decode, before its orientation, is compared sample by sample. This also chooses tier 1's JPEG decoder: the one nearer libjpeg-turbo's, which darktable's official build links.
- **colorin's profiles** as golden cases, on darktable's decoded image, as every port's case takes darktable's input ([0061](0061-golden-image-tests-against-darktable-cli.md)).
- **The whole pipeline** on JPEGs at their defaults and with tools set, as the exit test does on raws ([0062](0062-the-whole-pipeline-comparison.md)).

**The JPEG corpus,** downloaded or generated, never committed:

- JPEGs darktable-cli exports from the raw.pixls.us samples, in sRGB, Adobe RGB, Display P3 and one LUT-based ICC profile; with chroma at 4:4:4, 4:2:2 and 4:2:0, baseline, as darktable's writer encodes them; copies of one export with each of the eight EXIF orientations, the tag rewritten in place, since darktable writes orientation 1; the progressive cases from libjpeg-turbo's test images;
- libjpeg-turbo's own test images, under its BSD licence.

**The tolerances:**

| Check | Passes when |
| --- | --- |
| The decode | Every sample within 1/255 of darktable's, and at most 1% of samples off at all |
| colorin's profiles | The golden cases' `Tolerance::DARKTABLE` |
| The whole pipeline | The exit test's: a mean ΔE2000 of at most 0.01, at most 0.01% of pixels over 2.3 |

**After the migration,** darktable-cli stays the reference for decoders alone, a narrow exception to [0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md): what a file holds, compared on the decoded samples before any processing, never the processing, whose baselines are by then Apertrust's own. So tiers 2 to 6, after the migration, check their decoders against darktable-cli's decode by the decode's tolerance above, and their profiles and renders against Apertrust's own baselines. A format's first baselines are rendered at the commit that adds its decoder, by the pipeline whose own baselines pass there, and accepted on that pull request's evidence: its decode within the decode's tolerance of darktable-cli's on its corpus, and its renders shown. A first baseline replaces none, so it needs no decision of its own; every later change to it does.

### Placement

- **Tier 1, JPEG,** builds everything the later tiers reuse: the image kind and context, the upload and unpack, the non-raw order, colorin's profiles through moxcms, the defaults and tools, the library's photos, thumbnails and sidecars, and the golden checks. Planned for 0.2, within the migration. moxcms comes with the display colour's first part in 0.1, and is already in the tree through `image`.
- **Tiers 2 to 5,** each mostly a decoder and its profile's discovery, in 0.3 or later, TIFF and PNG first; **tier 6,** HEIF and HEIC, after its spike.

## Options considered

- **Raws only:** the maintainer requires non-raws.
- **ImageMagick or GraphicsMagick as darktable's catch-all, or `libvips`:** C, an exception to decision 5 for formats no photographer's workflow needs; the Rust decoders cover every format of darktable's own loaders.
- **A raw's JPEG listed as a photo of its own before step 17:** doubles the Browser of every photographer who shoots both, where darktable shows one photo for the two.
- **No darktable reference for decoders after the migration:** each format's own reference tool would be a different oracle for each tier, where darktable's decode is the one the migrated tools were matched on.
- **The `image` crate as the one decoder:** its JPEG is zune-jpeg, misreads one baseline layout (0.5.15), and its AVIF is dav1d, C; the crates it wraps are taken directly where they serve.
- **A new crate for the decoders:** `apertrust-raw` already reads the photo's file and its metadata, and no FFI or dependency direction asks for one ([0065](0065-names-and-crates.md)).
- **Uploading RGBA f32 from the CPU:** four times the bytes of an 8-bit JPEG over the bus and in memory; the unpack is one pass.
- **The raw order for non-raws, or linearising before it:** not darktable's; its JPEG order exists because the operations before colorin expect linear RGB.
- **lcms2's transform for LUT-based profiles, as darktable runs it:** a CPU path and C; moxcms's table on the GPU is measured against it.
- **The platform's own decoders for every format:** different samples on each computer, against the principles' same image everywhere.

## Consequences

- `apertrust-raw` gains the decoders and their metadata; its role in AGENTS.md and [crates.md](../crates.md) names them. `apertrust-plugin`'s image context gains `kind` and `profile`; the engine its upload, unpack and non-raw order; colorin its profile types through moxcms in `apertrust-color`; every tool's spec its page on a non-raw; the library its listing by the tiers' extensions; the packages their file types, registered never as the default handler, as darktable's are ([packaging.md](../packaging.md)).
- The planned crates are in [dependencies.md](../dependencies.md#images-other-than-raws-planned), each checked with `cargo deny check` when added; `avif-parse` is MPL-2.0, already allowed.
- Decisions 22, 26, 32 and 33 say what a non-raw changes in them; [engine.md](../engine.md), [library.md](../library.md), [plugins.md](../plugins.md) and [history.md](../history.md) describe it.
- Decision 2 states the decoders' exception; [testing.md](../testing.md) and AGENTS.md restate it where they give the rule after the migration.
- The golden harness gains the decode's check, colorin's profile cases and JPEGs in `pipeline`, with a script that fetches libjpeg-turbo's test images and exports the generated JPEGs.
- Open: HEIF and HEIC's decoder, among the four above, chosen after the tier's spike.
