# 0056. Export as an engine job

## Context

How does an export run, from the dialog and from the CLI, so that it writes what darktable would at the same settings, never makes the live photo wait, and never leaves a half-written file under a photo's name? An export renders on the same GPU as the viewport, and darktable's export settings, sizes and file names are what its users know.

## Decision

**An export is a `UserExport` job of the engine's session** ([0057](0057-jobs-as-darktables-control-jobs.md)), darktable's `dt_control_export`: one at a time, later ones queued; its body on a job worker; its renders `GpuTask`s that the engine thread steps after the live frame, one submission at a time ([0017](0017-threads-and-the-engines-session.md)).

- **Commands and events:** `Command::Export { photos, settings, places }`, `CancelJob`, `RetryJob` (planned) and `PreviewExport` for the dialog's lines; `Event::Job` for everything its row in the jobs window shows ([0072](0072-each-job-reports-its-own-row.md)), and `Event::Export`, the record for scripts and the CLI, with each photo's `Started` and `Done` (written, skipped as there or not changed, failed in plain words) and `Finished` with the counts. `places`, the person's folders and name, come from the client, since the engine reads no platform folder. Photos are named by path until the library's ids.
- **Each photo's edit as it is when Export is pressed:** the open photo's from the session's memory, every other photo's read from its sidecar by the edit writer in order with its writes, so the export holds every change sent before Export and none after.
- **Each photo in turn:** decoded on the worker; bound by `edit::instances`, colorout's profile the export's where it is not Image settings; planned at its size within the session's budget; rendered in tiles of at most 1024 × 1024, the operations before the tiles a step per pass, in row strips of about 8 MP (`export_job::STRIP_BINDING`), each tile one step, its readback mapped as the engine thread polls; quantised, encoded and written on the worker.
- **Failures:** a photo's own failure is that photo's and the job goes on, as darktable-cli's loop does; a failure of the destination ends the job, the rest reported not reached, as darktable's export job stops at a failed store. Cancel drops the photo in progress and its temporary file; the photos written stay. Quitting asks first while a job the person started runs.
- **Spike X1** measures a change's latency beside an export on each GPU, its mark a change within its time alone plus one submission of about 12 ms on the M4; where a step is longer, a render splits into submissions of consecutive passes.

**What an export writes and where lives in `apertrust-export`,** a crate with no GPU:

- **The size,** as darktable computes it (`export.c`, `control_jobs.c`, `dt_imageio_export_with_flags`, darktable-cli): a limit in pixels, centimetres or inches, or a scale as darktable parses it; with upscaling a side at 0 a hundred times the other in the dialog, the photo's own in the CLI; the format's largest side capping a limit; the scale the least of the limits over the output after geometry, at most 1 without upscaling; the output `floor(scale × W)` by `floor(scale × H)`, of the photo as its orientation turns it, the resample planned around the turn ([0016](0016-geometry-for-operations-that-move-pixels.md)). 0 is no limit, as darktable-cli has it. darktable's memory cap on the scale is left out, so one edit exports at one size on every computer (principle 2).
- **The resample** is darktable's `dt_interpolation_resample` with lanczos3, in the engine's `resample` kernel: without high quality after demosaic (after haze removal while it is planned), every later operation at the export's scale; with high quality or above 1 at finalscale's place before colorout; none at 1. Tiles are walked back through its plans, so a tiled export equals an untiled one bit for bit ([0013](0013-tiled-exports-equal-to-untiled-ones.md), [0015](0015-whole-image-passes.md)).
- **The writers:** JPEG through jpeg-encoder, pure Rust, its quality tables libjpeg's, darktable's Auto subsampling (4:4:4 above 92, 4:2:2 above 90, else 4:2:0) and the four others, optimised Huffman tables, the JFIF density, the profile in APP2; PNG through png at 8 or 16 bits, zlib levels 0 to 9 through flate2, iCCP and cICP as darktable's; quantised as darktable rounds. darktable's sRGB output profile is written by `apertrust-export` as lcms2 saves it, version 2.4, not by moxcms, whose encoder writes version 4 only.
- **The file on disk:** darktable's `variables.c` ported whole, its refusals named; darktable's disk storage (`_$(SEQUENCE)` added with several photos and no variable, a folder given `$(FILE_NAME)`, the folder made with its parents); On conflict's four (create unique filename, overwrite, overwrite if changed, skip); each file written to a temporary beside its name, synced and renamed, so a share never holds a part file under a photo's name.
- **Settings:** the `export.*` settings of a third owner, `export` ([0043](0043-settings-and-apertrusts-folders.md)), written by the app and read by the CLI as darktable-cli reads darktablerc, `--conf` over them; presets in `presets/export.toml` in the user data folder, read by the client, never the engine (planned).

**The CLI:** `apertrust-cli export FILE... -o PATTERN` runs a session in process and waits, with darktable-cli's `--width`, `--height`, `--hq` and `--upscale`, the extension choosing the format; `--out-ext`, `--icc-type` and `--preset` planned. `--runs`, `--tile`, `--skip`, `--set` and `--param` render directly.

**Planned:** the pipeline kept from photo to photo, rayon in the encoding, other profiles, metadata written into the file, styles, multi-preset export, the dialog on Windows and Linux.

## Options considered

- A job thread or a second device for exports: neither can put the live frame first, and a second device splits the budget.
- A separate process per export.
- Each photo's history read at its turn, as darktable does: a later change would leak into a running export.
- Stopping at every failure (darktable's job) or at none (darktable-cli).
- The viewport's box filter for exports, or a resample of the finished full-size export: unlike darktable's output.
- The image crate's JPEG encoder (4:4:4 only, standard Huffman tables); mozjpeg or turbojpeg (C).
- Writing in place; the dialog's settings kept by the app alone, which the CLI could not read; one file per preset.

## Consequences

- jpeg-encoder's IJG licence is allowed and acknowledged in `CREDITS.md` ([0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md)).
- A scaled export and a decoded JPEG against darktable-cli's are measured before they become golden cases.
