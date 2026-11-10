# 0077. A photo opens in stages

## Context

Opening a photo is one step on the engine thread: the raw decoded (135 to 245 ms at 61 MP on the M4), uploaded, its sidecar read, its kernels compiled (38 to 303 ms), its first frame rendered (302 to 388 ms for the 61 MP Sony 7R V). Until all of it is done the central panel says "Opening …" and the side panel shows every module at its defaults, drawn disabled; `Opened`, `Values` and the first `Frame` then come together. On WARP the first open compiles the raw stage's kernels for 34 to 39 s.

Opening should feel instant, in three stages: at once a picture of the photo, within milliseconds its module values, then the live frame. That raises four questions:

- how the picture and the early values reach clients, within decision 59's facts and notices;
- what the session does with an edit before the pipeline is live;
- where the picture comes from, keeping decision 43's rule that the engine opens no folder by itself;
- how quick switching through a folder behaves, with decision 57's cancel.

Principle 2 constrains the picture: what is not the engine's render of the edit must not be measured or worked on as if it were.

## Decision

**A photo opens in three stages,** each a state every client sees. Planned.

| Stage | When | Central panel | Side panel | Events |
| --- | --- | --- | --- | --- |
| 1. The picture | The click's frame, then within tens of milliseconds | The best picture in hand, fitted, unmarked | Every module at its defaults, drawn disabled, as now | `OpenPreview` |
| 2. The values | Milliseconds after the `Open` | The same picture | The photo's values from its sidecar, drawn disabled | `Opened`, `Values { live: false }` |
| 3. Live | Once the raw is decoded, uploaded and rendered | The live frame, in one swap; then scopes | The values, enabled | `Values { live: true }`, then `Frame` |

**The session's side.** `Open(path)` saves and closes the photo before it, cancels the earlier open's job, and adds a job of kind `open` on `SystemForeground`, with no progress, so it is never shown ([0057](0057-jobs-as-darktables-control-jobs.md)). Its body, on a job worker:

1. **The rendered thumbnail.** It reads the largest of the photo's rendered thumbnails in the cache, through the library's thumbnail cache, stale or not, since a reader may show a stale picture meanwhile ([0076](0076-the-cache-kinds-limits-and-control.md)). If it finds one, it sends it as `OpenPreview`.
2. **The metadata and the sidecar.** It maps the raw once for the whole open. It reads the raw's metadata without its pixels (a metadata read in `apertrust-raw`, rawler's decode with `dummy`), its EXIF, and its sidecar, after the edit writer's writes of that file, clamped as an opened edit is. It hands them to the engine thread, which sends `Opened(Photo)` and `Values { live: false }` while this open is still the latest.
3. **The embedded picture.** Where step 1 found no rendered thumbnail, it reads the camera's embedded picture at the longer side of the last `Resize`, never larger than the picture itself, and sends it as `OpenPreview`; where there is none, `OpenPreview` with no picture.
4. **The decode.** It waits for the session's one decode slot through `Context::wait`, so a cancel ends the wait, then decodes. A result whose open was cancelled meanwhile is dropped; otherwise the raw goes to the engine thread.

**The engine thread's side.**

- **Kernels.** As an open's decode starts, the engine thread plans its pipeline from the edit and the image context and compiles its kernels, so they are ready when the raw arrives. Kernels are kept across photos, keyed by kernel and entry point: decision 37's kernels for the session's life, built with this.
- **The first frame.** When the raw arrives for the latest open, it is uploaded and rendered. That batch sends `Values { live: true }`, then the `Frame` or the render's failure.
- **The early image context equals the decode's.** A corpus test holds them equal for every sample. Where they differ on a file, the session takes the decode's, its live `Values` reports the values over it, and `-d raw` logs the difference.

**The protocol** ([0059](0059-one-event-system.md)):

- **`Event::OpenPreview(OpenPreview { open, picture, source })`**, a fact, `Fact::OpenPreview`.
  - `picture` is 8-bit RGBA, upright, its pixels shared (`Arc<[u8]>`), as a thumbnail's; `source` is `Rendered` or `Embedded`; there is no picture where the photo has neither.
  - It is the open photo's picture until its first live frame. A client takes it only for its latest open and before that open's live frame, and `Sync` sends it only while the photo is not live. A remote client receives it encoded.
- **`Opened(Photo)`** comes at stage 2. It carries what the metadata, the EXIF and the sidecar give: the path, the open's number, the camera, the size at full resolution, `view_only`, the shooting data and the image context. The decode and upload times leave it, for `-d perf`.
- **`Values::live`** is false from `Opened` until the pipeline is live, and true from the batch of its first live frame. It is part of the fact, so a client joining late, or sent only the latest values, knows whether the photo is live.
- **A failure after `Opened`**, of the decode or the upload, is `Failed(Open { open, error })`, as before: the photo is closed and none is open.
- **Order.** The engine thread sends `Opened`, `Values { live: false }`, then `Values { live: true }` and the `Frame`. The job body sends `OpenPreview` through the outbox, as a thumbnail's job does, so only the open's number orders it against the rest. It is never sent after the job's cancel.
- **The CLI and MCP** see the same events. `export` and `preview` wait for the frame, as now. A client that edits waits for `Values { live: true }`, as the CLI's in-process helpers and the planned MCP adapter do.

**Until it is live, the photo is read-only.**

- Every command on the edit (`on_edit`), from an `Open` until its live values, is refused as `SessionError::Refused { command, error: NotLive }`, changing nothing and writing nothing.
- Commands not on the edit run as ever: `Resize`, `Scopes`, the jobs', `Export` and `Library`.
- The app draws the values disabled, so it sends no such command; the keys on the edit (a reset, later Undo) are disabled with them.
- An opening photo has no unsaved change, so a newer open closes it with nothing to write.

Refused, not queued:

- A queue would hold commands on the edit while later ones not on the edit run, so `through` could no longer say how many commands a report includes. Holding every command instead would hold a newer `Open`, which quick switching needs taken at once.
- A held change would apply to a photo the person has not yet seen rendered.
- An open that fails, or is replaced, would refuse the held commands anyway, only later.

**The picture for stage 1.**

- **In the frame of the click,** the app shows the best picture its mirror already holds for that photo: the largest thumbnail the grid panel has received, fitted. This is the app's own state, with no round trip. `OpenPreview` replaces it when it is larger or rendered.
- **The session's order:** the largest cached rendered thumbnail (at most 1024 pixels, step 6's), else the camera's embedded picture at the central panel's size, else none, the central panel saying "Opening IMG_2041.ARW".
- **The session fetches it,** not the app through the library's `Thumbnails`. So it comes with no folder open (one photo from a terminal), to every client, at the panel's size, and with the open's number. The cache is the handle the binary hands the session ([0076](0076-the-cache-kinds-limits-and-control.md)) and the raw is the file the `Open` names, so the engine opens no folder by itself ([0043](0043-settings-and-apertrusts-folders.md)).
- **No mark.** The picture is shown as it is, with no mark, badge or words over it. Until the live frame there is no scopes graph, no clipping overlay, no value under the cursor, no zoom past fit and no on-photo tool, and the swap to the live frame is one swap.
- **Two quiet hints instead:**
  - while the photo is not live, the disabled controls' tooltip says why, "Opening IMG_2041…", a message id with the file's name;
  - only if the photo is still not live about 1 second after the click (a slow share, lavapipe or WARP), the central bar shows a quiet "Rendering…" beside its other contents, never over the photo, gone when the live frame arrives. A fast open shows nothing.

**Switching quickly**, as with the arrow keys through a folder:

- **Every open is taken at once** and cancels the open before it: a queued job leaves at once (J14), and a running body stops at its next check. An older open's stage 2 reaching the engine thread late is dropped.
- **One decode at a time.** The newest open waits for the decode slot, and an older one still waiting is cancelled. A decode under way for a cancelled open runs to its end, since rawler cannot be stopped mid-decode, and its raw is dropped, never uploaded. So at most one raw is decoding beside the live one.
- **Each switch** shows the mirror's thumbnail in its click's frame, then its `OpenPreview` and its values. Only the open holding the decode slot decodes and compiles.
- **Clients** drop a replaced open's events by its number, as the app's mirror does.

**No low-resolution first render** between stages 2 and 3, for now.

- It cannot shorten the decode, which every render needs.
- From step 6 the rendered thumbnail already shows the edit's look.
- It would be darktable's preview-pipe shortcut that [0063](0063-departures-from-darktable-and-views-below-full-scale.md) rejects, and a second swap.
- **Measured first:** the first live frame's time split by pass (`Frame::timings`) on the M4, the RTX 5060, the Radeon 780M and lavapipe. A low-resolution first render could save at most the raw stage's and demosaic's share, less their cost at a quarter of the pixels.
- It is proposed as a decision of its own only if that saving is above 250 ms on a hardware GPU for a photo with no rendered thumbnail.

**Decision 37's open question, what a thumbnail shows before Apertrust's render, is settled.**

- Apertrust's rendered thumbnail of the photo's edit where the cache holds one, else the camera's embedded picture, else the file name.
- Unmarked everywhere: in the grid panel, as darktable shows its thumbnails, for the opening photo in the central panel, and in a still cell before its still (Phase 3).

**Timings and tests.**

- **Timings.** Each open logs its stages under `-d perf`: from the `Open` to `OpenPreview` with its source, to `Opened` and the values, to the decode's start and end, to the upload, and to the first live frame. The app logs its click to the first painted picture.
  - Recorded on the M4 and a software GPU (lavapipe, WARP when its workflow runs), and on the RTX 5060 and the Radeon 780M as they are reached.
  - For the 7R V (61 MP), the X-T5 (40 MP X-Trans) and the 40D (10 MP), with the file cache warm and cold, the session's first open and a later one, from a local SSD and from the NAS.
  - Quick switching: 20 photos at 30 a second, the last one's time to live, the decodes run and the peak memory.
- **Pass marks on the M4 from a local SSD:**
  - the picture in the click's frame where the mirror holds a thumbnail, else `OpenPreview` within 100 ms;
  - the values within 50 ms of the `Open`;
  - the live frame no later than today's one-step open.
- **Tests:**
  - an open's events in order and named by its number;
  - a command on the edit before live refused as `NotLive`, writing nothing, and taken after;
  - a second open before the first's decode ends, the first never uploaded and sending no live values or frame, one decode at a time (a decode held by a test hook, as `HoldFolder` holds a folder);
  - a decode failing after `Opened`;
  - the metadata read's image context equal to the decode's for every corpus sample;
  - `OpenPreview`'s order of sources;
  - in the app, the mirror and its kittest checks: the picture on the click, the values read-only, the swap and the controls enabled in one frame, a replaced open's picture dropped.

## Options considered

- **One step on the engine thread, as now:** the values, the picture and the frame arrive together, 0.5 to 0.9 s after the click at 61 MP on the M4 and over half a minute on WARP's first open, with the engine thread held throughout.
- **`Values` before `Opened`,** with a state saying the photo is not live: a client needs the photo's image context, which `Opened` carries, to show the views of its pending values, so `Opened` comes first.
- **A `Live` notice at stage 3** in place of `Values::live`: whether the controls take a change is state, which a notice neither resends to a client joining late nor keeps when a remote queue coalesces.
- **Edits queued until live:** `through` no longer counts what a report includes, or a newer `Open` waits; above.
- **Edits accepted at stage 2** against the metadata's image context and rendered once live: each change would be made against a picture that does not show it.
- **A preview mark on the opening photo and on stills:** the swap is usually under a second, a mark that flashes draws the eye to the one moment best not noticed, and a thumbnail is understood to be approximate.
- **The app fetching the picture** through the library's `Thumbnails`: nothing with no folder open, no other client gets it, it is capped at the grid's sizes, and it reads the raw beside the open.
- **A low-resolution first render:** above.
- **Several decodes at once** while switching: up to seven raws of 122 MB each in flight, slowing each other, for photos the person passed.

## Consequences

- **`apertrust-raw`** gains a metadata read without pixels, held equal to the decode's by a corpus test.
- **The session** gains:
  - the `open` job and the decode slot;
  - `Event::OpenPreview` and `Fact::OpenPreview`;
  - `Values::live` and `SessionError::NotLive`;
  - kernels kept across photos.
- **`Photo`** loses its decode and upload times, and decoding leaves the engine thread for the live photo too ([0017](0017-threads-and-the-engines-session.md)).
- **The app's mirror** gains the opening states, the central bar its "Rendering…" after about 1 second, and the editing tabs the read-only values with their tooltip. The CLI's helpers that edit wait for `Values { live: true }`.
- **On Linux,** rawler's `RawSource::new` populates the whole mapping as it is made, so the values wait for the whole file: the measurements say whether a mapping that is not populated is needed upstream. Compiling kernels off the engine thread, which WARP's 34 to 39 s holds today, is an open question ([engine.md](../engine.md#open-questions)).
- **The roadmap** lists it in Phase 2's folder mode, before the ways in (step 9b). The rendered thumbnail as stage 1's first source waits for step 6's rendered thumbnails; until then the embedded picture comes first.
