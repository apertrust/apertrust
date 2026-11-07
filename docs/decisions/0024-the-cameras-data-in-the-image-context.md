# 0024. The camera's data in the image context

## Context

Operations need facts about the camera beyond the mosaic: a DNG's colour matrix, the camera's name and ISO to find a noise profile, the lens corrections a raw carries, the camera's white balance presets. darktable reads these in its own ways and matches its data files by its own spellings. Where do they come from, in what form, and who reads them, when operations and modules may not depend on each other ([0028](0028-modules-bound-to-operations.md))?

## Decision

**`apertrust-raw` reads the camera's data and `apertrust-engine::raw` copies it into `apertrust_plugin::Camera`, part of the image context, as plain data.** Every operation, every module's binding and the app's pending display see the same values.

**A DNG's camera matrix, as darktable derives it** (`dt_exif_read`):

- `apertrust-raw` reads the colour calibration tags from the DNG's first IFD as recorded (CalibrationIlluminant1 to 3, ColorMatrix1 to 3, CameraCalibration1 to 3, AnalogBalance), since rawler's camera data keeps only part of them.
- `apertrust-color::dng_xyz_to_camera` holds darktable's light-source table and choice: scanning the calibrations in tag order, the one whose temperature is higher and no farther from 6500 K, light sources above 24 counted as D65, else the first unknown one with a matrix; then `CA · AB · CC · CM` in f32, `CA` darktable's Bradford matrix from the light source to D65 on the camera side.
- ForwardMatrix and the profile's maps are ignored, as darktable ignores them. Raws that are not DNGs keep rawler's matrix bit for bit; so does a DNG where no calibration qualifies.

**The camera's identity, as darktable's names:** `make`, `model`, `alias` and `iso`, meaning darktable's `camera_maker`, `camera_model` (the main name, an alias resolved), `camera_alias` (the name the file's camera has) and `exif_iso` (0 if unrecorded).

- Aliases are resolved from rawler's own embedded camera files (`rawler::decoders::CAMERAS_TOML`), parsed once with `toml`.
- darktable's spellings where they differ from rawler's (`Z 6_2`, `OM System`, `GFX100S`) are applied to the names in `apertrust-raw` (planned), so every user of darktable's data looks up by alias, then model.
- ISO by darktable's rule as far as rawler's EXIF reaches: ISOSpeedRatings above 0 (its second value where it holds two); at 65535 or missing, the tag SensitivityType names, in Exiv2's order; then, at 0 or 65535, RecommendedExposureIndex for Sony and Canon only. Maker-note ISOs stay a recorded difference.

**Lens corrections** the raw carries, as darktable stores them (`Camera::lens_correction`, an enum of darktable's Sony, Fujifilm, DNG and Olympus types holding `dt_image_correction_data_t`'s values after darktable's checks), read over rawler's public parsers ([0025](0025-lens-correction.md)). Every table and the automatic scale stay in the operation.

**White balance presets** (planned): `apertrust-raw` embeds darktable's `data/wb_presets.json` and parses it once with serde_json, by darktable's reading rules (names lowercased, a missing `tuning` read as 0), looks the camera up by `make` and `alias`, then `model`, and carries its entries in the file's order (`Camera::white_balance_presets`) for the temperature operation and the White balance module ([0027](0027-both-of-darktables-white-balances.md)).

**darktable's data files** live verbatim in the top-level `data/`, each embedded and parsed once by the crate that reads it, a test holding its SHA-256 ([0045](0045-a-top-level-data-folder.md)): `noiseprofiles.json` by the denoise operation, `wb_presets.json` by `apertrust-raw`.

## Options considered

- The DNG specification's own processing (interpolating between calibrations by the white balance, ForwardMatrix): moves every DNG away from the reference.
- Adding the missing tags beside rawler's matrix map: darktable's choice depends on the tags' order and on missing light sources, which the map loses.
- rawler's names with each operation resolving aliases; a generated alias table in an operation's crate: copies rawler's data and drifts.
- Raw tag bytes passed to operations, or the reader computing lens tables: file formats in operations, or darktable's maths outside them.
- `apertrust-color` parsing camera data; a crate of darktable's data.

## Consequences

- The three DNGs that failed the raw-stage checks match darktable.
- Open: the lens's identity, focal length, aperture and distance, which join with Lensfun's method; the highlight-preservation shift, which needs maker notes rawler does not read.
