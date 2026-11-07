# Port spec: raw black/white point (rawprepare)

## Source

darktable at the reference release, commit 20891f6c6f. The reference is darktable's CPU path.

- `src/iop/rawprepare.c`: `reload_defaults()`, `commit_params()`, `process()` and the region functions.
- The decoder's levels: `src/imageio/imageio_rawspeed.cc` (`dt_imageio_open_rawspeed`), and rawspeed at darktable's submodule commit c835b05 (`RawImageDataU16::calculateBlackAreas`, the decoders' black levels).
- OpenCL: `data/kernels/basic.cl` (`rawprepare_1f`).

## Status

Ported. The kernel is `slang/rawprepare.slang` in `crates/op/rawprepare`, with its operation and setup in `src/lib.rs`.

- **Ported:** the 16-bit mosaic path.
- **Not ported:** flat-field correction from embedded gain maps, and the floating-point and non-mosaic paths, which return `NotPorted`.
- **Against darktable:** with the same parameters, the kernel matches darktable within 7.2e-7 on 30 raws (Results).
- **Defaults:** rawler's crop and levels, which differ from darktable's own on most cameras (Defaults, decision 23).
- **GPUs:** checked on one, the Apple M4.
- **Module:** Raw black/white point is built ([../modules/raw-black-white-point.md](../modules/raw-black-white-point.md); Tool binding notes).

## Purpose

The first operation of the raw stage. It cuts the sensor's readout down to the developed area, dropping the masked borders and any unused photosites.

It scales each photosite so the sensor's black is 0 and its white point 1: `(raw - black) / (white - black)`. There is a black level for each of the four positions of the 2×2 pattern of row and column parities on the sensor.

Everything after it, white balance, highlight reconstruction and demosaic, assumes that scale. In the first pass its levels are the Raw black/white point module's, in the Base tab.

## Pipeline

- **darktable position:** first in every iop order (order 1.0), before invert, temperature and highlights.
- **Domain:** scene-referred, raw.
- **Input:** the sensor's 16-bit mosaic, uncropped (`Format::Mosaic16`).
- **Output:** the developed area's mosaic as `f32`, 0 at black and 1 at the white point (`Format::Mosaic`). Values below black stay negative and values above white stay above 1: nothing clips here.
- **Assumes earlier modules:** none; the decoder's crop and levels (Defaults).
- **Layer support:** background only, as for the whole raw stage.

## Parameters

darktable params version 2, 32 bytes: `left`, `top`, `right`, `bottom` (i32), `raw_black_level_separate[4]`, `raw_white_point` (u16), then two bytes of padding and `flat_field` (enum, i32).

Defaults are the image's (Defaults). Without a camera, the schema's defaults are darktable's "passthrough" preset (no crop, black 0, white 65535), since its introspection defaults put the white point at 0. Array elements take an `_<index>` suffix, so the CLI's `--set` needs no brackets.

| Name | darktable field | Type | Range | Default | Owner | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| Crop left, top, right, bottom | `left`, `top`, `right`, `bottom` | integer | 0 to 65535 | the decoder's crop margins | fixed: the sensor's geometry, not an edit | Photosites dropped from each edge of the readout |
| Black level 0 to 3 | `raw_black_level_separate[0..3]` | integer | 0 to 65535 | the decoder's black levels | Raw black/white point | The black level of parity position `(row & 1) * 2 + (column & 1)`, on the sensor |
| White point | `raw_white_point` | integer | 0 to 65535 | the decoder's white point | Raw black/white point | The raw value that becomes 1 |
| Flat field | `flat_field` | choice: `off`, `embedded` | | `embedded` when a DNG has four usable gain maps, else `off` | Raw black/white point | Multiplies each photosite by its gain map; not ported |

## Parameter versions

| Version | Change | Migration |
| --- | --- | --- |
| 1 | The crop, black levels and white point | Version 2 with `flat_field` off |
| 2 | Adds `flat_field` | |

Apertrust's schema is version 1 and holds version 2's fields. Golden sidecars carry version 2.

## Setup on the CPU (`commit_params`)

For a mosaic (`piece->pipe->dsc.filters` set): `sub[i] = black[i]` and `div[i] = white - black[i]`, as floats, for the four parity positions. Non-mosaic raws divide both by 65535 first (by 1 for floating-point HDR images); that path is not ported.

darktable also caches the mean black and the white point for later modules (`dt_iop_buffer_dsc_t::rawprepare`, used by highlight reconstruction and others). Apertrust passes them when a port needs them.

`flat_field` embedded applies the gain maps only if `_check_gain_maps` accepts them: four maps, one per Bayer position, each covering the whole image with at least 2×2 points and the same grid. `prepare` returns `NotPorted` for `embedded`.

`commit_params` also disables the module for an image that is not a raw, or a floating-point raw already normalised (white point 1). Apertrust's raw stage only runs on a raw's mosaic.

## Per pixel (`process`)

The 16-bit mosaic path, `rawprepare.c:process`:

1. The input photosite is at `(j + csy, i + csx)` in the input region, where `csx`, `csy` are the left and top margins scaled by `roi_in->scale / piece->iscale` (`_compute_proper_crop`). At full resolution they are the margins themselves.
2. Its parity position is `_BL`: `((row + roi_out->y + top) & 1) << 1 | ((col + roi_out->x + left) & 1)`, so position 0 is an even row and an even column of the sensor, not of the crop.
3. `out = (in - sub[id]) / div[id]`.

Apertrust's kernel reads the whole sensor's mosaic, two photosites to a 32-bit word, the even one in the low half. It computes the same position on the sensor from the pass's origin and the crop's corner.

The floating-point mosaic path is the same formula on `f32` input, and the third path scales a non-mosaic image per channel; neither is ported. The gain-map step multiplies the result by a bilinear sample of the photosite's position's map.

## Neighbourhood and scale

- **Neighbourhood radius:** none; per photosite.
- **Scale dependence:** darktable scales the crop margins when its preview pipe runs the raw stage on a downscaled buffer. Apertrust runs the raw stage at full resolution only (decision 12).
- **Tile overlap needed:** none.
- **Row strips** (decision 14): where the mosaics are larger than the GPU binds at once, the kernel runs in bands of rows. Each band binds the rows of the 16-bit readout and of the float output it covers, takes its first row from the pass header's origin, and indexes both whole mosaics less the header's `input_base` and `output_base`.
- **Region-of-interest changes:** the output is the input less the margins (`modify_roi_out`), and an output region needs the input region grown by them (`modify_roi_in`). `distort_transform` shifts points by the left and top margins.
  - Planning refuses a crop that leaves an empty image.
  - darktable instead warns about margins that leave less than half the sensor in either direction (`_image_set_rawcrops`) and still crops.

## Precision and special values

- darktable's maths is `f32` already. The division is darktable's; WGSL's is within 2.5 ULP, about 3e-7 at 1, the size of the differences the raw-stage run measures.
- `div` would be 0 where the white point equals a black level and negative where it lies below one. darktable's ranges allow this and its defaults never produce it; darktable then divides by zero, writing infinities and NaN, or inverts the mosaic. `prepare` keeps each `div` at least 1, one level, which is all a white point above the black can give, the levels being whole numbers. So the output stays finite and is darktable's bit for bit wherever the white point lies above the blacks (Deliberate differences).
- The input is integers, so there is nothing else to sanitise.

## CPU and OpenCL differences in darktable

None in the formula: `rawprepare_1f` computes the same position and the same division. The OpenCL kernels read the mosaic as an image rather than a buffer.

## Defaults

`reload_defaults` takes the image's crop margins, `raw_black_level_separate` and `raw_white_point`, which `dt_imageio_open_rawspeed` fills from rawspeed.

| Default | darktable (rawspeed) | Apertrust (rawler) |
| --- | --- | --- |
| Crop | rawspeed's crop, from its camera data's `Crop` or the format's own (a DNG's default crop) | the active area, every valid photosite; darktable's crop on only 6 of 33 raws (Results). The crop to the manufacturer's declared size comes after demosaic (decision 23) |
| White point | rawspeed's `whitePoint`, 65535 when it has none | rawler's first white level |
| Black levels | from the file for most formats, else measured (below) | the levels the raw stores, from the file or rawler's camera data; a measurement only when there are none (below) |

### Black levels

**darktable.** rawspeed's decoder sets `blackLevelSeparate` from the file for most formats: DNG tags, Canon maker notes on newer bodies, Nikon, Sony, Panasonic, Olympus, Pentax and Fujifilm data, or the camera data's per-position levels. When it does not, darktable calls `calculateBlackAreas`, which measures the camera data's masked areas if there are any and otherwise uses the single `blackLevel`.

That measurement has a bug darktable knowingly keeps (rawspeed issue 389):

- each row of a masked area contributes one photosite, from the crop's first column for a horizontal area and from the area's first column for a vertical one;
- each is counted once per column of the crop or of the area;
- the result is the median of those samples per parity position, from 16-bit histograms.

Apertrust reproduces it, bug and all, as `RawImage::masked_black` in `apertrust-raw`.

**Apertrust.** The defaults use the black levels the raw stores, and a measurement only when there are none: rawler's order, whose fallback is the mean of every masked photosite (decision 23).

The darktable-compatible measurement reproduces darktable's exactly on rawspeed's masked areas (the 40D: 1030, 1030, 1026, 1026). It serves `apertrust-cli info` and the defaults check, to explain darktable's levels, and never the defaults. Measured correctly, every masked photosite with the median per position, the 40D's black is 1025 to 1026 with no difference between rows, so darktable's row split is the bug's single column.

## Deliberate differences

- **No gain maps:** rawler does not read DNG opcode lists, so `flat_field` is never on by default and `embedded` is not ported. A DNG with gain maps (some phones') differs from darktable by the map's gain.
- **Stored black levels, never darktable's measurement:** on the cameras darktable measures (older Canons and a few compacts, whose raws store no levels darktable's decoder reads), the defaults differ from darktable's by a few units, visible only in the deepest shadows. An example is the 40D's 1023, which rawler reads from the maker notes, against darktable's 1030 and 1026. No raw of the corpus is one of them (decision 23).
- **Floating-point and non-mosaic raws are not ported:** the engine uploads only 16-bit mosaics.
- **Raw stage at full resolution only:** the margins never scale (decision 12).
- **A crop that leaves nothing is refused** at planning, where darktable warns and continues.
- **A white point at or below a black level** divides by one level, not by 0 or less.
  - `prepare` keeps each divisor, white less black, at least 1, so the photosites of that position scale to finite values, their raw value less the black. darktable writes infinities or NaN into highlight reconstruction, demosaic and everything after, or inverts them.
  - It sanitises the parameters at the operation's boundary, as the kernel rules ask (AGENTS.md).
  - Wherever the white point lies above the blacks nothing changes, so `raw-run` and `raw-defaults` give the same results. The unit test `a_white_point_at_or_below_a_black_level_gives_finite_output` and Raw black/white point's row R4 hold it.

## Tool binding notes

In the first pass (decision 2) the Raw black/white point module is darktable's: the four black levels, the white point and flat field correction as controls, the crop margins fixed (`docs/modules/README.md`).

The module is built (`crates/modules/src/raw_black_white_point.rs`, [../modules/raw-black-white-point.md](../modules/raw-black-white-point.md)). It owns every parameter:

- each level's control is named as its parameter and starts at this operation's default for the photo, which the module restates from the image context as `defaults` reads it, its tests holding the two together;
- it sets a level only off that default;
- it never sets flat field, which takes only off, or the crop margins.

The module shows the black levels and white point, as darktable's does, in raw units. The crop is the sensor's geometry and gets no control. A change re-runs demosaic (decision 12).

## Gradients

None: the raw stage exports no derivatives (decision 21). The formula is linear in the photosite and smooth in black and white away from `white = black`, so a later optimisation could differentiate it easily.

## Test plan

- **`apertrust-golden raw-run`:** rawprepare on the GPU with its defaults for each raw, against darktable's render with the same parameters written into its history, temperature at 1, 1, 1, demosaic's photosite-colour passthrough and an identity colorin (`raw_stage.rs`). Every photosite within 1e-5, at the same crop.
- **`apertrust-golden raw-defaults`:** darktable's own defaults, with rawprepare removed from the history, recovered from its render by fitting a line per parity position through the raw values; the crop is where the lines fit best. Compared with Apertrust's defaults.
- **Engine test** (`crates/engine/tests/pipeline.rs`): an odd-sized sensor with a crop at odd coordinates, against the formula on the CPU.
- **Unit tests:** the defaults from the camera's crop and levels, parity positions, the passthrough preset, the masked-area measurement and its bug.

## Results

Apple M4, darktable's five test raws and the 40 raw.pixls.us samples (`tools/fetch-raw-corpus.sh`). Of the 45, 33 are mosaics both decoders read. The rest are not mosaics (sRAW, linear DNGs, lossy DNG, the small ARW), use filters other than red, green and blue (`bayer4.crw`), or one of the decoders cannot read them.

### `raw-run`

30 of 33 match darktable within 7.2e-7 (mean differences about 5e-9), at the same crop, Bayer and X-Trans. The three that differ, by up to 1.1e-3 on about half their photosites, are Sony's lossy ARWs (`hlrecovery.arw`, the A1, the A6400), whose decoders seed their dither differently (decision 23).

### `raw-defaults`

darktable's own defaults against Apertrust's, which are rawler's:

| | Matches | Differs |
| --- | --- | --- |
| Crop | 6: 40D, 7D Mark II, D850 (twice), Z 6, 7R V | 27: the DNGs' default crop; a few pixels inside the active area on Fujifilm, Sony and Olympus raws, the R5, 5D Mark IV, D5100 and Z 9; beyond it or shifted on the 90D, M50, R6 Mark II, NX1 and K-3 Mark III; the whole readout on Panasonic's |
| Black | 28 | 40D (1030, 1030, 1026, 1026 against 1023), NX1 (128 against 0), R6 Mark II (511 against 512), M50 (510, 510, 511, 511 against 510, 511, 511, 511), Pixel 4a (gain maps) |
| White | 28 | R6 Mark II (14008 against 12735), Z 9 (15892 against 15520), K-3 Mark III (16315 against 16375), OM-1 Mark II (4096 against 4095), Pixel 4a (gain maps) |

The fits are exact to rounding (residuals about 1e-7) except on the Pixel 4a, whose embedded gain maps darktable applies by default, and the lossy ARWs.

### The differences judged

The defaults are rawler's camera data, with each difference judged on the evidence (decision 23):

| Difference | Verdict |
| --- | --- |
| R6 Mark II's white | a rawler bug, fixed upstream after 0.8.0 |
| R6 Mark II's black, OM-1 Mark II's white | rawler is right |
| Panasonic's crop | the file's own |
| NX1's black | deliberate in rawler but wrong for that camera; a question for upstream |
| K-3 Mark III's white | no bug (below) |
| Z 9's white | a rawler bug (below) |
| Every crop | by design: Apertrust's final image is the declared size |

**The K-3 Mark III's white.** rawler's 16375 is fixed camera data that agrees with the maker notes' WhiteLevel, 16376. In `IMGP9854.PEF`, 12% of whose photosites sit at the 14-bit ceiling, green and blue saturate anywhere from about 16250 up to 16383. So rawler's white and darktable's 16315 both lie inside the sensor's own spread, and highlight reconstruction's level, 0.987 of the range (16165), lies below it.

**The Z 9's white.** rawler's 15520, with a 12-bit mode the Z 9 does not have, was copied from the Z 6 and Z 7 when dnglab added the Z bodies in one commit (2ee0964014). rawler's Z 8, the same sensor and processor, and every Nikon added since have 15892, as do rawspeed and Adobe's DNG Converter.

- Neither value is where the sensor clips. In the lossless ISO 64 sample green stays proportional to red and blue within 1% up to the 14-bit ceiling and clips hard at 16383, without the spread of a saturating sensor.
- So 15892 is a margin of 0.047 EV, one that also covers the High Efficiency modes, whose clipped areas decode a little below the ceiling.
- No source makes the white depend on ISO.
- The fix, 15892 without the 12-bit mode, is drafted for upstream. Until it arrives the Z 9 comes out 2.6% brighter than in darktable.

## Open questions

- Decoder differences to report upstream: the Z 9's white (a fix), the NX1's black and crop and the 40D's masked band (questions), drafted for rawler (decision 23).
- Gain maps: the corpus's Pixel 4a needs them to match darktable's default render; rawler does not read DNG opcode lists.
