# Port spec: orientation

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/flip.c` (`commit_params()`, `process()`, `modify_roi_out()`, `modify_roi_in()`, `distort_transform()`, `distort_backtransform()`, `reload_defaults()`), with `dt_imageio_flip_buffers` from `src/imageio/imageio.c`, `dt_image_orientation_t`, `dt_image_orientation` and `dt_image_orientation_to_flip_bits` from `src/common/image.h`, and the EXIF reading in `src/common/exif.cc`. The OpenCL kernel is `flip` in `data/kernels/basic.cl`. The reference is darktable's CPU path.

Status: ported, every orientation, in Slang (`crates/op/flip`), as an operation with its manifest, schema, `prepare` and `geometry` (`Flip`, `FlipGeometry`): the first real operation that moves pixels (decision 16). It is in `OPERATIONS` with `default_enabled` on, as darktable's `reload_defaults` enables it on every image, at auto. At no rotation it runs no pass, and the planner leaves it out of the plan, as darktable's `commit_params` disables its piece, so a photo the right way up plans exactly as it did before the port. Its module is Orientation ([../modules/orientation.md](../modules/orientation.md)). Checked on one GPU so far.

Golden results (mire1.cr2, 3908×2602, Apple M4), through the operation's `prepare` as a one-instance plan:

| Case | darktable test | Orientation | ΔE2000 mean | max | One render (median) |
| --- | --- | --- | --- | --- | --- |
| `flip-ccw` | 0005-orientation-minus-90 | rotate 90° (6) | 0 | 0 | 3.7 ms |
| `flip-cw` | 0006-orientation-plus-90 | rotate -90° (5) | 0 | 0 | 3.6 ms |
| `flip-180` | 0007-orientation-180 | rotate 180° (3) | 0 | 0 | 3.6 ms |
| `flip-horizontal` | 0005, edited | flip horizontally (2) | 0 | 0 | 3.7 ms |
| `flip-vertical` | 0005, edited | flip vertically (1) | 0 | 0 | 3.6 ms |
| `flip-transpose` | 0005, edited | transpose (4) | 0 | 0 | 3.7 ms |
| `flip-transverse` | 0005, edited | transverse (7) | 0 | 0 | 3.7 ms |

Every case is darktable's output bit for bit: the tolerance is none (`cases::FLIP`).

The exit test (`apertrust-golden pipeline`, Apple M4), with orientation at auto on both sides: 27 of the 29 colour mosaics pass, as before the port, the two Sony lossy ARWs failing as they did. The X100VI's raw, which its EXIF turns, is compared upright, 5152×7728: mean ΔE2000 0.001, max 13.68, 0.0003% of pixels over 2.3, the same as with orientation off on both sides. With the vignette case added (`--add vignette`), the turned X100VI is 0.064 mean ΔE2000 from darktable, where the X-T5, landscape, is 0.079 and the X100VI with orientation off on both sides 0.083: the vignette frames the turned photo as darktable's does.

## Purpose

Turns or mirrors the photo in steps of 90°, so it shows the right way up: by default the photo's own orientation, which the camera records in the EXIF Orientation tag, and otherwise the one the user chooses. Algorithmically, a permutation of whole pixels, no interpolation: each output pixel is one input pixel. It is the Base tab's Orientation module (`docs/modules/README.md`).

## Pipeline

- **darktable position:** 16.0 in the v5.0 order, after rotate and perspective (15.0) and haze removal (14.0), before enlarge canvas (16.5), crop's predecessor clipping (17.0), retouch and exposure (21.0).
- **Apertrust slot:** camera RGB 16, darktable's place, between haze removal (14) and exposure (21) (decision 22; `docs/engine.md`, Order).
- **Domain:** scene-referred: it moves linear values and computes nothing.
- **Input and output:** RGBA in camera RGB (`IOP_CS_RGB` by darktable's `default_colorspace`); every channel moves with its pixel, alpha included.
- **Assumes earlier modules:** demosaic. darktable's `IOP_TAG_DISTORT`: every later operation sees the turned image, its size and its positions (Neighbourhood and scale).
- **Layer support:** background only: it changes the image's geometry, which a layer cannot.

## Operation

| Manifest field | Value |
| --- | --- |
| `id` | `flip` |
| `version` | 1 |
| `slot` | camera RGB, 16 (decision 22) |
| `domain` | scene |
| `layers` | false |
| `differentiable` | false: no continuous parameter (Gradients) |
| `radius` | 0 |
| `default_enabled` | true, darktable's `reload_defaults` |
| `provenance` | darktable, commit 20891f6c6f; `src/iop/flip.c`, `src/imageio/imageio.c`, `src/common/image.h` |

## Parameters

darktable params version 2 (`DT_MODULE_INTROSPECTION(2, dt_iop_flip_params_t)`, `flip.c:42`): `orientation`, a `dt_image_orientation_t` stored as an `int32`.

| Name | darktable field | Type | Range | Default | Meaning | Owner |
| --- | --- | --- | --- | --- | --- | --- |
| `orientation` | `orientation` | choice | darktable's nine values, -1 to 7 | auto (-1, `ORIENTATION_NULL`) | how the image is turned: auto, the photo's own orientation, or one of darktable's eight | Orientation |

`dt_image_orientation_t` (`image.h:134`) is three flip bits, `ORIENTATION_FLIP_Y` (1), `ORIENTATION_FLIP_X` (2) and `ORIENTATION_SWAP_XY` (4), applied in that order: the rows flipped, the columns flipped, then the axes swapped. The schema's choices are darktable's values from -1 by their `$DESCRIPTION`s, the choice at index i darktable's i − 1:

| Choice | darktable | Value | `$DESCRIPTION` |
| --- | --- | --- | --- |
| `auto` | `ORIENTATION_NULL` | -1 | autodetect |
| `none` | `ORIENTATION_NONE` | 0 | no rotation |
| `flip_vertically` | `ORIENTATION_FLIP_Y` | 1 | flip vertically |
| `flip_horizontally` | `ORIENTATION_FLIP_X` | 2 | flip horizontally |
| `rotate_180` | `ORIENTATION_ROTATE_180_DEG` | 3 | rotate 180° |
| `transpose` | `ORIENTATION_SWAP_XY` | 4 | transpose |
| `rotate_cw_90` | `ORIENTATION_ROTATE_CW_90_DEG` | 5 | rotate -90° |
| `rotate_ccw_90` | `ORIENTATION_ROTATE_CCW_90_DEG` | 6 | rotate 90° |
| `transverse` | `ORIENTATION_TRANSVERSE` | 7 | transverse |

## Parameter versions

| Version | Change | Migration |
| --- | --- | --- |
| 1 | `orientation` an `int32` holding the user's flip relative to the image's orientation | to version 2 by `merge_two_orientations` (`flip.c:114`, `legacy_params` at 143): the image's orientation, its FLIP_X and FLIP_Y bits exchanged where the user's swaps the axes, XOR the user's |
| 2 | the orientation itself, the image's included | current |

The golden cases hold version 2; `ports.rs` refuses version 1, which needs the image's orientation as darktable loads it. The import of darktable's edits will need it (decision 51, planned).

## The photo's orientation

darktable reads it as it imports the raw (`exif.cc:1737`): the EXIF `Exif.Image.Orientation` tag, IFD0's, else a Panasonic raw's `Exif.PanasonicRaw.Orientation`, each turned into flip bits by `dt_image_orientation_to_flip_bits` (`image.h:532`): EXIF 1 none, 2 flip horizontally, 3 rotate 180°, 4 flip vertically, 5 transpose, 6 rotate -90° (clockwise), 7 transverse, 8 rotate 90° (anticlockwise), any other none. `dt_image_orientation` (`image.h:526`) reads an image that records none as none. A Sinar back's orientation in its thumbnail's IFD (`exif.cc:1748`) is not read.

Apertrust reads the same tag through rawler's EXIF metadata, which reads tag 0x112 from the IFD darktable's Exiv2 calls IFD0 (or a Panasonic raw's root, PanasonicRaw's), into `apertrust_raw::RawImage::orientation`; rawler's decoders set their own orientation only for a DNG, so a Fujifilm raw taken upright read as normal until this port. The engine turns it into darktable's flip bits (`apertrust_engine::raw::flip_bits`) in the image context's `Camera::orientation`. Of the raw.pixls.us corpus, the Fujifilm X100VI's `DSCF0074.RAF` is rotate 270° (EXIF 8), the iPhone 12 Pro's DNG rotate 90° (6), and the rest normal; the GoPro HERO5's GPR (2) is a file rawler cannot decode.

## Setup on the CPU (`commit_params`)

`flip.c:410`: auto takes the photo's orientation (`dt_image_orientation`), any other value is its own; and where the result is none, the piece is disabled (`piece->enabled = FALSE`). `apertrust_op_flip::orientation` restates the first part. For the second, `prepare` returns no pass, its output its input, and the planner leaves an operation that moves no pixel and runs no pass, keeping its input as it is, out of the plan (`pipeline.rs`, `add`). So a photo the right way up has no orientation in its plan, its bytes and cache keys those it had before, as the geometry tests' fingerprints of six plans without geometry hold.

Otherwise `prepare` returns one pass, the kernel's `main`, with the input region's width and height and the flip bits (`FlipParams`, 16 bytes after the header), and an output of the output region's size: the input's sides swapped where the axes swap.

## Per pixel (`process`)

`flip.c:354` turns its whole input region with `dt_imageio_flip_buffers` (`imageio.c:860`): input pixel (i, j) of a region `wd` by `ht` lands at x = `wd` − 1 − i where the columns flip, else i, and y = `ht` − 1 − j where the rows flip, else j, the two then swapped where the axes swap, in an output `ht` wide. The kernel gathers instead: each output pixel (x, y) of its region reads the input pixel (i, j) with (i, j) = (y, x) where the axes swap, else (x, y), then i = `iw` − 1 − i where the columns flip and j = `ih` − 1 − j where the rows flip (`source` in `flip.slang`), and copies its four channels. The input region is exactly the output region turned back (below), so the kernel needs no position.

## Neighbourhood and scale

- **Neighbourhood radius:** none; the manifest's radius is 0.
- **Geometry** (decision 16), `FlipGeometry`, darktable's four hooks:
  - `extent`, `modify_roi_out` (`flip.c:299`): the input's sides, swapped where the axes swap.
  - `input_region`, `modify_roi_in` (`flip.c:315`): the bounding box of the output region's four corner pixels turned back by `backtransform` (`flip.c:183`) in the input stage at the pipe's scale. darktable sizes that stage `buf_out × roi_out->scale`, its sides swapped back; Apertrust takes the planner's `Extent::scaled`, rounded once for every stage, so a quarter turn swaps the sides exactly. A permutation of whole pixels, the region is exactly the output region's pixels.
  - `forward`, `distort_transform` (`flip.c:214`): on the input's full size, x → W − x where the columns flip, y → H − y where the rows flip, then the swap.
  - `backward`, `distort_backtransform` (`flip.c:249`): the swap, then the flips on the input's sides.
- **Scale dependence:** none of its own. It runs after the viewport's downscale at the pipe's scale, as every geometry does (`docs/engine.md`, Geometry, Where it runs).
- **Tile overlap needed:** none. A tile's input region comes back through `input_region`. A first round of tiles before it, haze removal's, is of that operation's own stage, the photo before the turn.
- **At a size** (decision 56; `docs/ports/finalscale.md`): a geometry of whole pixels (`Geometry::whole_pixels`), so the export resamples in the stage its place gives, as darktable's does: without high quality after demosaic in the photo's own stage, its sides the output's turned back, the turn after it, as darktable's `modify_roi_in` takes the export's region back through the turn; with it at finalscale's place in the turned image. Each stage's size at the export's scale is the output's taken back through the turns after the resample, a turn's extent its own inverse.
- **Fusion:** its pass is declared per pixel, though it reads another pixel than it writes; the fuser takes no pass of an operation that returns a geometry, so a run ends at it (`docs/engine.md`, Fusion).
- **What later operations see:** the turned image. The pass header's full size is each operation's stage at full resolution, darktable's per-module `buf_in`, and whole-image passes cover their stage at the pipe's scale (decision 16), so vignetting centres itself on the turned photo, as darktable's does. Soften's and the tone equalizer's radii come from the pipe's input, the raw's readout, as darktable's `piece->iwidth` and `iheight` do; neither turns.

## Precision and special values

None. It moves bits and computes nothing: no fp64 in darktable, no clamp, no division; NaN and infinity move as they are.

## CPU and OpenCL differences in darktable

darktable's OpenCL `flip` (`basic.cl`) scatters each input pixel to its turned place, the CPU's map; the kernel here gathers by its inverse, the same permutation.

## Deliberate differences

- **The input stage's size at the pipe's scale:** `input_region` turns a region back on the planner's `Extent::scaled`, every stage's extent times the fitted scale rounded once, where darktable's `modify_roi_in` takes `buf_out × roi_out->scale` truncated to an integer. They can differ by a pixel in the viewport alone, where the scale is not 1, moving the turned region by that pixel; an export at full size and at a size, whose stage sizes are the output's taken back exactly, agree with darktable. Rounded once, a quarter turn swaps the sides exactly (decision 16).
- **The planner leaves the identity out:** darktable disables the piece at no rotation; Apertrust's operation returns no pass and the planner leaves it out. The same render; it is no departure.
- **The photo's orientation from rawler's EXIF reading,** not Exiv2's: the same tag in the same IFD. A Sinar back's thumbnail orientation is not read.
- **darktable's legacy user flip** (`reload_defaults`, `flip.c:479`), the orientation an old darktable library database kept beside the image, has no source in Apertrust, which has no such database.

## Tool binding notes

The Orientation module owns `orientation`, darktable's page: the transform label with darktable's four buttons inline, its values the schema's, written as the same index once off auto ([../modules/orientation.md](../modules/orientation.md)). **What is an identity:** no rotation, and auto on a photo that records none; then the operation leaves the plan. **darktable's defaults for a new instance:** auto, on every photo (`reload_defaults`), and its "auto" preset auto-applies it (`init_presets`, `flip.c:442`). The module is not switched: darktable enables it on every image. darktable's presets are auto, no rotation, flip horizontally, flip vertically, rotate by -90 degrees, rotate by 90 degrees and rotate by 180 degrees, waiting for the header's presets menu. darktable's buttons, rotate 90° anticlockwise and clockwise (`do_rotate`, `flip.c:525`) and flip horizontally and vertically (`_flip_h`, `_flip_v`, 563 and 579), write the orientation from the one rendered, auto read as the photo's, through `Module::action`, sent as `SetControl`; they also tell crop to follow (`_crop_callback`), which comes with crop.

## Gradients

None: the one parameter is a choice of eight permutations, with nothing continuous to differentiate, so the kernel exports neither `flip_backward` nor `flip_forward_diff` (`Kernel::derivatives` false, the manifest's `differentiable` false), and `apertrust-golden gradient` reports the cases as having no gradient port. The derivative of its output with respect to its input image is the inverse permutation, a gather like `main`, with no atomics; it comes with the derivatives with respect to the image, which no operation exports yet.

## Test plan

- **Golden cases:** darktable's tests 0005, 0006 and 0007 (rotate 90°, rotate -90°, rotate 180°), and test 0005 edited to each other orientation, flip horizontally and vertically, transpose and transverse, since darktable's own mirroring tests (0008 to 0010) mirror through crop. Each runs as a one-instance plan; the harness takes the output's size from the plan, which an operation that moves pixels changes.
- **Tolerance:** none, mean and max ΔE2000 0 (`cases::FLIP`): a permutation of darktable's input renders darktable's output bit for bit.
- **The exit test:** `apertrust-golden pipeline` keeps darktable's orientation on at auto on its side, as Apertrust's default, and crops darktable's render to the declared image turned the same way. The X100VI's raw, turned by its EXIF, is checked upright on both sides. At a size, `pipeline X100VI --width 2048 --height 2048` measures 1365×2048 at mean ΔE2000 0.001, max 1.90, with high quality, and 0.002, max 2.42, without, none over 2.3; the X-T5, landscape, at the same size 0.002, max 1.46 and 1.81.
- **The engine** (`crates/engine/tests/geometry.rs`): through the default pipeline, every orientation's export is the export the right way up with darktable's buffer flip, bit for bit, Bayer and X-Trans, and its tiles the whole export; a photo the right way up plans without orientation, and one its EXIF turns plans it, with its sides swapped, in the export and the viewport; the vignette after every orientation renders what it renders on the image flipped beforehand, bit for bit, whole and in tiles; an export at a size of a turned raw without high quality is the turned export of the photo the right way up at the swapped size, bit for bit, and with high quality or without its tiles are the whole export; haze removal's first round before every orientation tiles as it runs whole, at full size and at a size. The session (`session.rs`): a photo its EXIF turns opens upright, its frame the level photo's turned pixel for pixel, its size the turned photo's fitted (`Values::extent`, `Values::viewport_size`).
- **The operation** (`crates/op/flip`): auto is the photo's orientation, the region map exactly the output region's pixels turned back by darktable's buffer flip, the point maps each other's inverse and the buffer flip's on pixel centres, no rotation no pass.
- **Invariants:** the module's (`docs/modules/orientation.md`).

## Open questions

1. **Haze removal's full-resolution estimate** is offered only in a plan without geometry, so on a turned photo the viewport estimates at its own scale (`docs/engine.md`, Geometry, Still to build, item 2). A photo the right way up keeps it.
