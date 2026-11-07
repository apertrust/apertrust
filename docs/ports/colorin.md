# Port spec: input colour profile (colorin)

Source: darktable (the reference release, commit 20891f6c6f), `src/iop/colorin.c` (`reload_defaults()`, `commit_params()`, `process()` through `process_cmatrix` and `_cmatrix_fastpath_simple`), with `src/common/colorspaces.c` (`dt_colorspaces_create_xyzimatrix_profile`, `_create_lcms_profile`, `dt_colorspaces_get_matrix_from_input_profile`) and lcms's `cmsCreateRGBProfile`, and `src/common/exif.cc` (`dt_exif_read`), where a DNG's embedded matrix comes from. The reference is darktable's CPU path.

Status: ported. The kernel is `slang/colorin.slang` in `crates/op/colorin`, its setup `src/lib.rs`, the colour maths in `apertrust-color`, a DNG's matrix in the engine's image context. Ported: the camera's matrix (darktable's standard and embedded matrices), linear Rec2020 and Rec709 as input, linear Rec2020 as the working space, temperature's late correction; gamut clipping, blue mapping, ICC files and the other profiles return `NotPorted`. Where the two decoders' matrices agree, it matches darktable to a mean of about 3e-8 (Results). Checked on one GPU so far.

## Purpose

Takes the demosaiced image from the camera's own RGB, its filters' primaries, to the working space, linear Rec2020, where every later operation works. For a raw it uses the camera's matrix, XYZ to camera RGB under D65. In the first pass the Input colour profile module, darktable's input color profile, owns every colorin parameter, the camera profile included (`docs/modules/README.md`).

## Pipeline

- **darktable position:** after exposure (order 28.0), before color calibration.
- **Domain:** scene-referred; camera RGB in, working RGB out. darktable outputs Lab and converts to the working space for the next RGB module; Apertrust outputs the working space directly.
- **Assumes earlier modules:** demosaic; temperature's preset decides the late correction (decision 21).
- **Layer support:** background only.

## Parameters

darktable params version 7: `type` (profile type), `filename`, `intent`, `normalize` (gamut clipping), `blue_mapping`, `type_work`, `filename_work`. Apertrust leaves out the file names, which name ICC files on disk.

| Name | darktable field | Type | Default | Owner | Meaning |
| --- | --- | --- | --- | --- | --- |
| Input profile | `type` | choice, darktable's 27 profile types | `standard_matrix` for a raw | Input colour profile | Where camera RGB's primaries come from |
| Intent | `intent` | choice | `perceptual` | Input colour profile | lcms rendering intent; matrix profiles ignore it |
| Gamut clipping | `normalize` | choice: `off`, `srgb`, `adobe_rgb`, `lin_rec709`, `lin_rec2020` | `off` | Input colour profile | Not ported |
| Blue mapping | `blue_mapping` | bool | false | Input colour profile | Not ported; only old edits turn it on |
| Working profile | `type_work` | choice | `lin_rec2020` | Input colour profile | Only linear Rec2020 is ported, the working space the operations after it assume |

## Setup on the CPU (`commit_params`)

- **The camera matrix**, XYZ to camera RGB under D65: for a raw that is not a DNG, darktable's standard matrix, from its decoder's camera table; Apertrust's is rawler's D65 matrix, or its last (decision 23). For a DNG, darktable's default is the embedded matrix, `d65_color_matrix`, which `dt_exif_read` (`exif.cc`, lines 2026 to 2279) derives from the file's tags in its first IFD; Apertrust's image context derives it the same way (`apertrust-color::dng_xyz_to_camera`, from the tags `apertrust-raw` reads):
  1. It reads CalibrationIlluminant1 to 3, ColorMatrix1 to 3 when they have nine values, CameraCalibration1 to 3 when they have nine values and their ColorMatrix does, and AnalogBalance when it has three. ForwardMatrix is read only to tag the image "no samples"; the profile's hue and saturation maps, look table and tone curve are not read.
  2. A light source above 24, the EXIF "other" (255) among them, counts as D65.
  3. Scanning the calibrations in the order of their tags, it chooses one whose colour temperature, from darktable's table, is higher than the chosen one's and no farther from 6500 K: D65 itself, else the nearest, ties going to the higher temperature. A nearer light source of lower temperature after a farther one of higher temperature does not replace it. With no light source known, it takes the first ColorMatrix as D65's. Two calibrations on either side of D65 are not interpolated, as the DNG specification asks; darktable tags such a file "no samples".
  4. `d65_color_matrix = CA · AB · CC · CM` (`mat3mul`, `f32`): the ColorMatrix, the CameraCalibration and the AnalogBalance as a diagonal, each the identity when missing, and `CA`, darktable's Bradford matrix from the light source to D65 (Bruce Lindbloom's tables), on the camera side of the matrix.

  darktable's choice departs from the DNG specification in two ways, and Apertrust follows darktable, its reference: its adaptation to D65 multiplies the camera side of the matrix (`CA · CM`), where a chromatic adaptation of its XYZ side would be `CM · CA⁻¹`, and it chooses one calibration where the specification interpolates between two by the white balance.

  temperature's D65 coefficients start from the same matrix (`dt_colorspaces_conversion_matrices_rgb` takes the embedded matrix before the standard one). A diagonal CameraCalibration or AnalogBalance scales the matrix's rows, which leaves the camera profile's primaries where they are and moves only the D65 coefficients, so only the late correction shows it; `CA` moves both.
- **The camera profile** (`dt_colorspaces_create_xyzimatrix_profile`): the camera matrix inverted (camera RGB to XYZ); the chromaticities of its columns become an RGB profile's primaries, with darktable's D65 (XYZ 0.95045471, 1, 1.08905029) as white; lcms scales the primaries so camera white is that D65 and Bradford-adapts the result to D50. The luminance of the columns is dropped, so only the matrix's chromaticities matter.
- **The working profile:** Rec2020's primaries with `D65xyY`, (0.31271, 0.32902), Bradford-adapted to D50 the same way. The two D65s differ in the fifth decimal, so the adaptations do not quite cancel; Apertrust reproduces both (`apertrust-color::camera_to_xyz_d50`, `profile_rgb_to_xyz_d50`).
- **The matrix:** working ← XYZ D50 ← camera, composed in `f64`.
- **The late correction:** with temperature's preset "as shot to reference", colorin multiplies camera RGB by the camera's D65 multipliers over its as-shot ones (`chr->D65coeffs / chr->as_shot`), so the image reaches the working space at the camera's D65 reference, and the saturation level moves with it (decision 21).

## Per pixel (`process`)

`out = M · (in ∘ correction)`, alpha untouched. darktable computes `in ∘ correction`, converts to XYZ D50 with the camera profile's matrix, to Lab, and the pipe later back to XYZ and the working space; the Lab round trip only rounds.

## Neighbourhood and scale

None; per pixel.

## Precision and special values

`f32` matrices, as darktable's. No clipping: out-of-gamut colours stay negative, as in darktable's unbounded mode.

## Deliberate differences

- **rawler's camera table:** darktable's standard matrix is its decoder's camera table's; Apertrust's is rawler's (decision 23). Where they differ, the result differs (the 40D's in the fourth decimal). A DNG's embedded matrix comes from the file on both sides.
- **A DNG whose chosen calibration has no three-colour ColorMatrix** (a four-colour camera's, or a light source tag without its matrix): darktable composes its matrix with the identity in the ColorMatrix's place; Apertrust's image context has no DNG matrix and falls back to rawler's, which a four-colour camera does not have either. When no calibration qualifies at all (only light sources 5 to 8, which the DNG specification leaves undefined), darktable falls back to its standard matrix and Apertrust to rawler's matrices.
- **No Lab between colorin and the working space.**
- **Only the camera matrix and linear Rec2020 and Rec709 inputs;** ICC files, gamut clipping and blue mapping are not ported.
- **No derivatives:** colorin has no continuous parameters to optimise.

## Test plan

- **`apertrust-golden raw-run`:** the raw stage and colorin with their defaults, against darktable with the same raw-stage parameters and its own default input profile (the embedded matrix for DNGs, the standard matrix otherwise), with temperature's preset as Apertrust has it on both sides: its default, "as shot to reference" wherever the camera has a colour matrix (decision 27), so the late correction runs too, or with `--as-shot` "as shot", without it.
- **Unit tests:** white stays white; linear Rec2020 in is the identity; the late correction is D65 over as shot and moves the saturation level; what is not ported. A DNG's matrix (`apertrust-color`): the calibration chosen per light source in darktable's scan order, the adaptation to D65, CameraCalibration and AnalogBalance in darktable's order, and the matrices darktable prints (`-d imageio`) for three corpus DNGs. The image context (`apertrust-engine`): a raw that is not a DNG keeps rawler's matrix, a DNG gets darktable's, its tags count only at darktable's sizes, and a DNG with no qualifying calibration falls back to rawler's.

## Results

Apple M4, `raw-run` on darktable's five test raws and the 40 raw.pixls.us samples: 33 mosaics are compared; the others are linear or monochrome DNGs, or one of the two decoders cannot read them.

**"As shot" on both sides** (`--as-shot`): 29 of 33 match darktable to a mean under 4e-7 (X-T4 2.9e-8, 5D Mark IV 1.5e-8), with as few differing pixels as demosaic alone.

**"As shot to reference" on both sides**, temperature's default (decision 27), so the late correction runs: the other raws' matrices are rawler's, and 29 of 33 match in this condition too. The four mosaic DNGs, the only raws whose matrix comes from the DNG handling:

| DNG | Calibrations | darktable's matrix | As shot | As shot to reference |
| --- | --- | --- | --- | --- |
| 5D Mark III, Adobe DNG Converter, lossless | A and D65; CameraCalibration 1.002, 1, 1.0062; AnalogBalance 1 | D65's, rows scaled by the CameraCalibration | 1.7e-8 | 1.9e-8 |
| Pixel 4a | A and D65; CameraCalibration2 1.034, 1, 0.9764; AnalogBalance 1 | the same | 6.9e-7 | 7.4e-7 |
| Q2 | A and D50 | D50's, adapted to D65 | 1.9e-8 | 1.8e-8 |
| K-1 Mark II | A and D65; AnalogBalance 1 | D65's as it is | 3.2e-8 | 3.2e-8 |

A diagonal CameraCalibration shows only with the late correction (Setup). The lossy 5D DNG and the iPhone 12 Pro's are linear and the M Monochrom's is monochrome, so `raw-run` skips them; the lossy 5D's matrix comes from its tags as the lossless one's does, and the other two keep rawler's. The Pixel's 0.004% of pixels over 1e-4 are the same with and without the late correction.

With highlight reconstruction in the chain (decision 15), the four DNGs match darktable to a mean of 1.7e-8 to 3.2e-8 with either preset (`raw-run` with each); once its clipped highlights, 1.7% of its photosites, are reconstructed, the Pixel 4a has under 0.00005% of its pixels over 1e-4, against 0.004% without.

The raws that differ more:

| Raw | Mean \|Δ\|, as shot (to reference) | Why |
| --- | --- | --- |
| 40D | 2.2e-6 (4.0e-5) | rawler's camera matrix and darktable's decoder's differ in the fourth decimal (0.607 against 0.6071) |
| Sony's three lossy ARWs | 6.8e-5 to 1.5e-4 (7e-5 to 1.6e-4) | their decoders seed the dither differently (decision 23) |

## Open questions

- **The next rawler release** (dnglab PR #891, merged 2026-10-29) models a DNG's camera profiles and holds CameraCalibration and AnalogBalance itself; `apertrust-raw` can then take them from rawler instead of reading the first IFD, keeping darktable's choice and composition (decision 24).
