# Port spec: demosaic

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/demosaic.c` (`reload_defaults()`, `commit_params()`, `process()`), with `src/iop/demosaicing/ppg.c` (`demosaic_ppg`), `rcd.c` (`rcd_demosaic`), `xtrans.c` (`xtrans_markesteijn_interpolate`) and `vng.c` (`_vng_lininterpolate`). OpenCL: `data/kernels/demosaic_rcd.cl` and `demosaic_markesteijn.cl`, whose per-pixel form the kernel follows. The reference is darktable's CPU path.

Status: ported.

- **Code:** the kernel is `slang/demosaic.slang` in `crates/op/demosaic`, with its operation and setup in `src/lib.rs`.
- **Coverage:** only RCD and Markesteijn 1-pass, darktable's defaults for Bayer and X-Trans sensors: RCD inside the band of PPG darktable keeps at the edges, Markesteijn 1-pass inside darktable's band of linear VNG. The other methods return `NotPorted` (decision 51), VNG4 and Monochrome among them, darktable's defaults for a pattern of four colours and a raw with no colour filters, which are refused; so do green equilibration, colour smoothing, dual demosaicing and capture sharpening.
- **Checks:** with the same parameters the raw stage matches darktable to a mean difference of about 1e-8 (Results). Checked on one GPU so far.
- **Module:** Demosaic (`docs/modules/demosaic.md`), darktable's page in the Base tab, its defaults the operation's, the methods and controls not ported drawn disabled.

## Purpose

The last operation of the raw stage: each photosite measured one colour, and demosaic estimates the other two from its neighbours, turning the mosaic into an RGB image in the camera's colour space. RCD (ratio corrected demosaicing, Luis Sanz Rodríguez) interpolates green along the more uniform of the vertical and horizontal directions, using colour ratios, then red and blue along the more uniform diagonal. Markesteijn's X-Trans algorithm interpolates in four directions and keeps, per pixel, the most homogeneous ones in YPbPr. Its module is Demosaic, darktable's, in the Base tab (`docs/modules/demosaic.md`).

## Pipeline

- **darktable position:** after temperature and highlight reconstruction (order 8.0), before colorin.
- **Domain:** scene-referred; raw in, camera RGB out.
- **Input and output:** the white-balanced mosaic, `f32`, the active area, in (`Format::Mosaic`); RGB, the manufacturer's declared crop, out (`Format::Rgba`, alpha 0 as darktable writes it). The passes run over the whole mosaic and only the passes that write the output crop it, so demosaic has real neighbours at the edges of the declared image (decision 23).
- **Assumes earlier modules:** rawprepare and temperature: RCD reads the processed maximum temperature leaves (decision 21).
- **Layer support:** background only.

## Parameters

darktable params version 6: `green_eq` (enum), `median_thrs` (f32), `color_smoothing` (enum), `demosaicing_method` (enum), `lmmse_refine` (enum), `dual_thrs`, `cs_radius`, `cs_thrs`, `cs_boost` (f32), `cs_iter` (int), `cs_center` (f32), `cs_enabled` (gboolean).

| Name | darktable field | Type | Range | Default | Owner | Meaning |
| --- | --- | --- | --- | --- | --- | --- |
| Method | `demosaicing_method` | choice, darktable's 17 methods | | darktable's `reload_defaults`: `rcd` for Bayer, `markesteijn` for X-Trans, `vng4` for a pattern of four colours, whatever its size, and `monochrome` for a raw with no colour filters, as darktable's monochrome sraw (Open questions) | Demosaic | The algorithm; only RCD and Markesteijn 1-pass are ported, and the module lists the sensor's methods (decision 30) |
| Match greens | `green_eq` | choice: `no`, `local`, `full`, `both` | | `no` | Demosaic | Green equilibration; not ported |
| Edge threshold | `median_thrs` | f32 | 0 to 1 | 0 | Demosaic, not ported: PPG only | PPG's pre-median |
| Colour smoothing | `color_smoothing` | choice: `off` to `five` | | `off` | Demosaic | Median passes on colour differences; not ported |
| LMMSE refine | `lmmse_refine` | choice | | `median` | Demosaic, not ported: LMMSE only | |
| Dual threshold | `dual_thrs` | f32 | 0 to 1 | 0.2 | Demosaic, not ported: dual methods only | |
| Capture sharpening | `cs_enabled`, `cs_radius`, `cs_thrs`, `cs_boost`, `cs_iter`, `cs_center` | bool, f32, int | darktable's | off | Demosaic, not ported | Not ported |

`commit_params` falls back to the sensor's default when the method is for the other kind of sensor (an X-Trans method on Bayer gives RCD, and the reverse Markesteijn); `prepare` does the same.

## Parameter versions

Version 2 (`green_eq`, `median_thrs`), 3 (adds smoothing, method, LMMSE refine), 4 (dual threshold 0.2), 5 (capture sharpening's parameters), 6 (`cs_enabled`). Apertrust's schema is version 1 and holds version 6's fields; golden sidecars carry version 5.

## Setup on the CPU

`prepare` builds the passes and their planes. RCD reads its input over the processed maximum (`dt_iop_get_processed_maximum`: the largest channel's, at least 1) and scales back. Markesteijn's set-up is darktable's: a hexagon of eight offsets around each position of the 3×3 repeat (`allhex`) and the solitary green's position, computed from the pattern as it sits at the mosaic's corner on the sensor (decision 21).

## Per pixel

**Bayer, `rcd_demosaic`:** `demosaic_ppg` over the whole mosaic first (the outermost three photosites from each colour's mean in their 3×3 neighbourhood; green elsewhere by PPG's gradients; red and blue from the green), then RCD's steps 0 to 4.3 (populate, vertical and horizontal discrimination, low-pass filter, green at red and blue, diagonal filters and discrimination, red and blue), whose result replaces PPG's 9 or more photosites from the edges (`RCD_MARGIN`). darktable runs RCD only when the mosaic is at least 20 photosites in both directions.

The kernel follows the CPU loops' bounds exactly, and their planes: the half-width `lpf` and `PQ_Dir` share one plane, so step 4.2 reads low-pass values where step 4.1 wrote no direction, and the diagonal filters are stored for odd columns only, so step 4.1 reads them at asymmetric neighbours; both packings use an even row stride, as darktable's tiles of 112 have. The vertical and horizontal discrimination is 0 within 4 photosites of the edges, as in darktable's zeroed tile planes.

**X-Trans, `xtrans_markesteijn_interpolate` with one pass:** the green bounds of each red and blue pair, green in four directions, red and blue at solitary greens (six steps, alternately horizontal and vertical, the better of the two by steepness for the last two directions), red at blue and blue at red, red and blue for the 2×2 blocks of green (the first two directions only, as darktable fills them), YPbPr (ITU-R BT.2020), second derivatives along each direction, homogeneity counts against 8 times the smallest derivative, their 5×5 sums, then the mean of the directions within an eighth of the most homogeneous. `_vng_lininterpolate` then replaces everything within 12 photosites of the edges: each colour's mean in the 3×3 neighbourhood on the outermost photosites, a weighted mean of each colour's neighbours (2 along the axes, 1 on the diagonals) inside them.

The CPU's tiles pad the mosaic with mirrored photosites; nothing that reaches the result 12 or more photosites from the edges depends on them, so the kernel computes over the mosaic alone.

## Neighbourhood and scale

- **Neighbourhood radius:** 10 for RCD, 12 for Markesteijn 1-pass (darktable's tiling borders); the manifest gives 12.
- **Scale dependence:** none: demosaic runs at full resolution, and the viewport downscales after it (decision 12).
- **Region-of-interest changes:** the output is the declared crop of the input; the passes run over the whole input (`Pass::size`, decision 21).

## Precision and special values

- darktable's maths is `f32`; RCD's divisions are guarded by `eps` (1e-5) and `epssq` (1e-10), as here. Input is clamped at 0 before RCD and Markesteijn, and the output never negative.
- Direction choices compare floats (`|0.5 − VH| < |0.5 − neighbourhood|`, homogeneity thresholds), so a rounding difference occasionally tips one, changing a small cluster of pixels by up to a few hundredths: the source of the rare differences below.

## CPU and OpenCL differences in darktable

darktable's OpenCL RCD computes the discrimination planes over a wider area than the CPU, and its Markesteijn green bounds take both photosites of a pair's hexagons where the CPU's walk takes the first one's. The kernel follows the CPU in both; the corpus confirms it (Results).

## Deliberate differences

- **Only RCD and Markesteijn 1-pass are ported** (decision 51): PPG alone, AMaZE, VNG4, LMMSE, VNG, Markesteijn 3-pass, frequency domain chroma, the passthroughs, Monochrome and the dual methods return `NotPorted`, as do green equilibration, colour smoothing and capture sharpening. Mosaics under 16 photosites, for which darktable switches to VNG, do too.
- **A pattern of four colours and a raw with no pattern are refused** (`prepare`: colour filters other than red, green and blue; monochrome raws), so their darktable defaults, VNG4 and Monochrome, which the operation's defaults follow, never render.
- **The declared crop** (decision 23): darktable writes the whole active area, with PPG or VNG at its edges; Apertrust writes the manufacturer's size, so on most cameras the edge bands are cropped away.
- **Contrast sensitivity's default** is the params' 0.4. darktable's `reload_defaults` computes `cs_thrs` from the photo (`_get_variance_threshold`, `src/iop/demosaicing/capture.c`, line 49): 0.07 lower for a raw white point above 4096, then 0.012 lower for each 100 ISO below 600 and higher above, the ISO within 100 to 1000. Capture sharpening is not ported, so it never renders.
- **Memory:** RCD's planes take about 26 bytes per photosite and Markesteijn's 96 (darktable's CPU path works in tiles); the engine's tiling bounds them.

## Tool binding notes

The Demosaic module is darktable's (decision 2; `docs/modules/demosaic.md`), in no group of darktable's scene-referred preset, so in Base, after Rotate and perspective, the group darktable's "modules: all" gives it (decision 38): the method, the per-method thresholds and LMMSE refine, colour smoothing, match greens and the Capture sharpen toggle, each shown where darktable's `gui_changed` shows it, and capture sharpening's five controls, darktable's collapsible section, a collapsible group shown while the toggle is on (decision 29), in the raw stage where darktable has it (`docs/modules/README.md`). It owns every parameter, its defaults the operation's for the photo, so an untouched module sets nothing; what is not ported is drawn disabled and refused. A change re-runs demosaic (decision 12).

## Gradients

None: the raw stage exports no derivatives (decision 21).

## Test plan

- **`apertrust-golden raw-run`:** rawprepare, temperature and demosaic with their defaults on the GPU, against darktable's render with the same parameters and its own demosaic, compared over the declared crop. Passes when under 0.05% of pixels differ by more than 1e-4 and the mean difference is under 1e-6. Rounding tips the occasional direction choice, most often Markesteijn's homogeneity ties across flat, clipped highlights (the X100VI's differing pixels have a median green of 0.995, at the white point). A porting error shows as a mean above 1e-6 and whole percents of pixels, as the lossy ARWs' different input does.
- **Engine tests:** a grey scene, white balanced, demosaics to grey everywhere, edge bands and interior, Bayer and X-Trans, at the declared crop.
- **Unit tests:** the default method by sensor, the passes and planes, the pattern's colours from the mosaic's place on the sensor, the X-Trans hexagons and solitary green, what is not ported.

## Results

Apple M4, darktable's five test raws and the 40 raw.pixls.us samples: rawprepare, temperature and demosaic with their defaults against darktable with the same parameters, over the declared crop (`raw-run`).

| Sensor | Raws | Mean \|Δ\| | Pixels over 1e-4 | Largest \|Δ\| |
| --- | --- | --- | --- | --- |
| Bayer, RCD | 25 | 8e-9 to 8e-7 | 0.0000% to 0.0042% | 1.2e-3 to 6e-1 (Pixel 4a) |
| X-Trans, Markesteijn | 5 | 2e-8 to 3e-7 | 0.0030% to 0.0194% (X-T5), 0.0130% (X100VI) | 5e-3 to 3e-2 |

Every edge band matches exactly: PPG's 9 photosites around RCD and the 12 of linear VNG around Markesteijn. The differences are isolated clusters in the interior where a direction choice tips. The three Sony lossy ARWs differ from the start (decision 23), by 4e-5 to 1.4e-4 on average after demosaic; they are not counted above.

## Open questions

- A raw with no colour filters: the image context gives `cfa: None` to every raw that is not a mosaic, a linear raw (the corpus's lossy Adobe DNG, iPhone 12 Pro DNG, Canon sRAW and Sony small lossless ARW) as well as one of a monochrome sensor (the Leica M Monochrom). darktable reads the first as a colour sraw, demosaic not applicable and `reload_defaults` RCD, and only the second, of one component a pixel, as its monochrome sraw, Monochrome. The context cannot tell them apart yet, so the defaults take the monochrome sraw's; settle it when the pipeline first takes either. Highlight reconstruction's spec notes the same gap.
- Memory: RCD's and Markesteijn's planes at 50 to 100 megapixels, where the engine's tiling has to bound them.
