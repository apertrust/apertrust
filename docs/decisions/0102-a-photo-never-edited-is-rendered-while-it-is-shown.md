# 0102. A photo never edited is rendered while it is shown

## Context

Once Apertrust has rendered a photo live, its render is the photo's thumbnail at every size, whatever `library.raw_thumbnails_from` says ([library.md](../library.md#thumbnails)). The live photo's thumbnail is a still of its viewport pipe (`Pipeline::still`, [engine.md](../engine.md#rendered-thumbnails-of-the-live-photo)). A still made as a photo leaves hurries its reads of the old pipe, so they run beside the next photo's open.

The pass mark for flipping through photos is this: 20 raws, each opened as soon as the one before is live, and the slowest `Open` to first live frame no more than one still's turn over the same flip with no still. The test missed it on the AMD Radeon 780M (integrated, shared system RAM, Vulkan, Linux), with the raws on a local NVMe drive. The medians below are of photos 2 to 12:

- Nikon Z 6 (24 MP): no change beyond the runs' spread.
- Sony 7R V (61 MP): 785–869 ms with no still, 825–886 ms with it. That is 30 to 45 ms slower, against a mark of about 7 ms (the still's slowest turn).

The GPU timestamps of the next photo's decode, upload, pipeline build and first frame were unchanged. The time went into the next open's CPU work: the metadata read (rawler's parse) went from 122–132 ms to 161–172 ms. Delaying the departing still by 200 ms moved the same loss to the decode.

These were ruled out: the GPU queue (the still yields to live frames), the disk and the JPEG writes (skipping them changed nothing), the memory budget (no eviction and no wait; a 3307 MiB live pipe and a 42.7 MiB still) and the job queue. The likely cause, not yet proven, is shared memory bandwidth and the shared power budget. The still's downscale reads the 61 MP raw stage's image, about 930 MiB, while the CPU parses and decodes the next raw.

So on an integrated GPU, any GPU work that runs beside an open slows it, however the work takes turns on the queue. The question is when a photo never edited is rendered, so that flipping through photos costs nothing.

## Decision

**A photo never edited is rendered while it is shown, never as it leaves.**

- **Never edited** means opened with no edit (no sidecar, or one at the defaults) and not edited since. A photo reset, its history discarded or taken back to Original in the session keeps the edited path below, so leaving it within a second never leaves the old edit's render in the Browser.
- **When.** Its still is due `UNEDITED_SETTLE`, 300 ms, after its first live frame drawn, which waits for the viewport's size; none is tried before it. It runs while the photo is on screen and yields to editing as every still does (`EDITING_QUIET`).
- **No departure still for it.** A photo never edited that is left before its still has begun gets no render. Its slots keep what `library.raw_thumbnails_from` shows: under Never, the camera's embedded picture, until the photo is opened again and shown for 300 ms. The old pipe's buffers go as soon as the next photo's open needs them.
- **A still already under way** when the photo leaves is hurried and finished, as before (`State::depart`, `Sources::hurry`).
- **Edited photos are unchanged.** An edit's still comes `THUMBNAIL_SETTLE`, one second after its last saved write, or as the photo leaves if its saved edit changed. Only the live pipe holds what the edit renders.
- **The pass mark,** restated:
  - **Gated.** In a back-to-back flip (each photo opened as soon as the one before is live, a dwell of 0), the slowest `Open` to first live frame is within the runs' spread of the same flip with no thumbnail work. It is measured on the Radeon 780M and on the M4.
  - **Reported, not gated.** The same flip with a dwell just past `UNEDITED_SETTLE`. A photo left while its still runs pays part of the still there.
- **Built** in `session/thumbnail/rendered.rs` (`UNEDITED_SETTLE`, `Open::unedited`, `Open::shown_at`), with its tests and a dwell in the ignored test `time_flipping_photos` (`APERTRUST_FLIP_DWELL_MS`). On the Radeon 780M the back-to-back flip renders nothing and is within the spread of the flip with no thumbnail work at 24 and 61 MP; with a 350 ms dwell, 19 of 20 photos are rendered and a 61 MP flip is about 10 ms slower in the median. The gate passed on the Radeon 780M and on the Apple M4, for both raws (engine.md).

## Options considered

- **Keep the departure still and accept the miss** as a cost of integrated GPUs until the M4 is measured. Not chosen: the 780M is a supported GPU, and integrated GPUs on shared memory are what most laptops have. The M4 is unified memory too, so it would measure the size of the same effect, not its absence.
- **Hold the departing still until the next photo is live.** Not chosen: in a back-to-back flip the cost only moves to the photo after next. It also keeps the old raw stage, about 930 MiB at 61 MP, beside the next photo's 3.3 GB pipe through its upload, which `Sources::hurry` exists to avoid.
- **No departure still, the still still at one second** (`THUMBNAIL_SETTLE`). Not chosen: most photos in ordinary culling, half a second to two seconds each, would keep the camera's picture.
- **Make the still cheaper**, downscaled from the fitted view's image rather than the raw stage's (about a thirteenth of the reads). Not chosen: the still would no longer equal the live frame at 1024 pixels byte for byte, and a second path would need tests of its own.

## Consequences

- A photo flicked past in under 300 ms keeps the camera's picture under Never. Before its first live frame, the Viewer showed only the opening picture, the camera's own, so the Browser shows what the person saw.
- engine.md › Rendered thumbnails of the live photo gives the trigger, and › Flipping through photos the restated mark and its numbers. library.md › Thumbnails says "opened" is "shown at least 300 ms".
- On an integrated GPU, an edited photo left within a second of its last change still has its still beside the next open. That cost is accepted: no other source holds its edit.
