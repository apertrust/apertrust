# Architecture decisions

Each architecture decision has a file of its own here, named `NNNN-short-title.md`. A record says what is decided now: the context, the decision, the options considered and the consequences.

- **To add a decision,** take the next number, write a new file with the same four sections, add a line to its area's table in the index, and update the area's row in [the current state by area](#the-current-state-by-area).
- **To change a decision,** edit its record so it states what is decided now, and update the docs and code comments that describe it.
- [architecture.md](../architecture.md) is the overview of the design, and links each area's document, which holds its details and its open questions.

## The current state by area

All major technology choices are made; one waits on a spike to be confirmed, the AI models on the WebGPU execution provider. Each row gives the current state of one area, citing its decisions by number; the records below hold the options and reasons. Update the row with the record.

| Area | Decision | Rejected |
| --- | --- | --- |
| Product | A free, non-destructive, cross-platform raw photo editor; eight principles in three groups, Trust, Ease and Help, which product decisions follow ([principles.md](../principles.md); 1) | A darktable rewrite with a friendlier UI as the identity; agents as the headline |
| Migration | darktable's colour science and modules migrated first, as darktable has them, in Apertrust's interface; the migration ends once, for the whole project, at the end of Phase 3, after which Apertrust evolves on its own with its own renders as baselines (2) | A per-module end; darktable as the reference for good |
| Reference release | darktable's latest stable release; its number lives in one constant of the golden harness and in each manifest's provenance (3) | darktable's master |
| Modules and operations | Modules are what people edit; operations are the engine's algorithms and never appear in the UI; each parameter has one owning module (4) | An Expert tab with direct operation access |
| Language | Rust (5) | C++ |
| C and C++ libraries | None compiled in or shipped unless no Rust option exists; ONNX Runtime the one exception, loaded at run time; libraries the platform provides, loaded at run time, outside the rule (5) | lcms2, exiv2, lensfun or RawSpeed compiled in |
| Programs and platforms | Windows, macOS and Linux, first-class; two executables, `apertrust` and `apertrust-cli` (6) | Separate server and MCP programs |
| Packaging | cargo-packager from one config, `packaging/Packager.toml`: an app bundle and disk image, an NSIS installer and a zip, an AppImage and a tarball, each with the CLI and the licences; signing and notarisation planned (68) | A script per platform; WiX; cargo-dist |
| GPU | wgpu over Vulkan, Metal, DX12 and WebGPU; a GPU-resident fp32 pipeline on storage buffers (8) | A hand-written Vulkan and Metal layer; host copies between operations |
| Without a GPU | The same kernels on a software GPU, lavapipe or WARP; what cannot bind there is refused, naming the pass and the sizes; image-shaped scratch bound in windows before Phase 3's exit (7, 15) | A CPU version of every operation; Slang's CPU target |
| Kernels | Slang compiled to WGSL, committed and validated with naga; every differentiable kernel exports derivative entry points (9) | Plain WGSL |
| Direct3D 12 | FXC pinned in the engine; no kernel writes a local at a run-time index (10) | DXC, a C++ library to ship |
| Differentiable pipeline | Differentiable-ready from the start; a command-line test matching a reference photo is Phase 3's first engine work (11) | Forward-only, with per-module pickers |
| Planning and caching | Regions propagated backwards; the raw stage and demosaic at full resolution, downscaled for the viewport; a few cache points; a rebuild keeps compiled kernels and matching cache points (12, 28) | Every operation over the whole image; caching every output |
| Tiles and strips | Exports tiled after the raw stage, bit-identical to untiled ones (13); a pass larger than one binding in row strips, the viewport's demosaic in tiles (14) | The raw stage tiled from CPU uploads |
| Whole-image passes | Results of any size written into scratch at the pipe's scale, never read back; a first round of export tiles after demosaic; haze removal's estimate at full resolution (15) | A pass kind per result size |
| Views below full scale | The export's processing at the view's scale, within a measured target, rather than darktable's preview shortcuts; iterated box means with radii r and r + 1 (63) | darktable's preview pipe |
| Departures from darktable | Only by an agreed difference, listed in its port's spec, darktable's behaviour kept behind a setting only the golden harness reaches (63) | Departing wherever Apertrust's maths seems better |
| Geometry | Each operation's `Geometry`, darktable's region and point maps, interpolated in its kernel (16) | Hardware filtering |
| Pipeline order | darktable's v5.0 order; every slot unique, its number rising along the pipeline (22) | Reordering by the user |
| Fusion | Chains of per-pixel operations fused into one kernel from Phase 2, the mechanism still open; one pass per operation until then (12, 22) | — |
| Threads | A UI thread; an engine thread owning GPU submission, its frames in two display buffers; one submit gate shared with the window (17, 18) | Rendering on the UI thread; a second device |
| Opening a photo | In three stages: the best picture in hand at once, a cached rendered thumbnail or the camera's embedded picture, unmarked; the values from the sidecar and the raw's metadata before the decode, read-only, commands on the edit refused until live; then the live frame in one swap; a newer open cancelling an older, one decode at a time (77) | One blocking step; a preview mark; edits queued until live; a low-resolution first render before measuring |
| History and undo | One log of steps per virtual copy, saved in the sidecar; a step one completed change with its source, note and batch, each module's whole state after it, darktable's undo merging a control's quick repeats; Undo and Redo taking back the session's undo records as darktable's, else moving the log's end; `Undo`, `Redo`, `GoTo`, `Compress`, `Truncate`, `Reset` and `Step` through the session, one whole `History` fact ([history.md](../history.md); 47, 78, planned) | Undo in memory only; steps as deltas; a history event of deltas |
| Scopes and readouts | Counted on the GPU after each frame, read back without waiting, sent as events (19) | Readback and binning on the CPU |
| Colour pickers | One active picker per session, a module's or the Colour picker module's, with stored samples beside it; regions in the photo's coordinates, kept in the original image's; reduced on the GPU by the engine's own kernel at the operation's input and output in the fitted viewport pipe and on the frame; a reading applied by the session through the module's pure `pick` as one change of source Auto, only after a move; one overlay language for every mark on the photo, darktable's line; the Colour picker module in the History tab ([pickers.md](../pickers.md); 79). Built: 79.1, the Colour picker's own picker through the session, measured on the frame; the rest planned | The client applying samples; a readout per operation; a host copy of the box; the zoomed view's region |
| Operations | Native plugins, one crate each (Slang source, setup, manifest, schema), built in at compile time; third-party ones later as declarative packages with WebAssembly logic (20, 21) | Loading darktable modules; native shared-library plugins |
| Raw decoding | rawler, its camera data trusted and fixed upstream; the camera named as darktable names it (23, 24) | RawSpeed, LibRaw |
| Lens correction | The corrections raws carry, first; then lensfun's database, its lookup ported to Rust and its models in Slang (25, 5) | lensfun through FFI |
| Colour management | moxcms, validated against lcms2 in CI; operations see the edit's output profile, never the display's (26) | lcms2 shipped |
| White balance | Both of darktable's: White balance over `temperature`, and Colour calibration with its CAT tab, As shot snapped as darktable snaps it (27) | One White balance driving colour calibration |
| Module layer | Static definitions and pure bindings in `apertrust-modules`, run by the engine's session; `.apxmp` version 1 holds module values (28) | Each client translating module values |
| Module pages and controls | darktable's page: its tabs and collapsible groups, no Advanced toggle (29); darktable's sliders (hard and soft ranges, factor, digits, step) and the options it lists per photo, Exposure darktable's slider, `exposure.exposure` itself over darktable's −18 to +18 EV, its default the photo's (30); lossless views, preset rows and curves of the same stored values (31) | Sliders narrower than darktable's; every option on every photo |
| Switched modules | A module darktable leaves off on a new photo stays off until touched, then takes darktable's defaults; its reset turns it off (32, planned) | A power button; darktable's defaults as a preset |
| Tone mapping | One tone mapper on the background per edit, a module of its own: AgX now, sigmoid and filmic rgb to come; a layer's own tone mapper replacing it within the layer's mask, planned with layers (33) | A fixed AgX; stacked tone mappers |
| Layers | Masked module instances at their pipeline positions, in a fixed order; each slider scaled in its own unit; a mask's parts combined top to bottom, an empty mask changing nothing; retouch as layers listed between the adjustment layers and Image, applied first; darktable's blend modes; masks and retouch ported from darktable's master (34, 74, Phase 4) | Compositing at the end of the pipe; darktable's linear mix for every slider |
| UI toolkit | egui, one interface for the desktop and a desktop browser (35) | Iced, Slint, GPUI, Tauri with web panels |
| Design system | `apertrust-ui`: tokens and themes in TOML, IBM Plex Sans with Plex Mono for the log's lines, Fluent and ICU4X; British and American English, Ukrainian and Polish; AccessKit on (36); the slider's marker a setting, Triangle, Circle or Square, Square by default, whole at the track's ends (75) | egui's default look; gettext; darktable's triangle alone, cut off at the ends |
| One view | No separate views: the Photos tab chooses photos, the centre shows the Photo layout and later Map, the grid panel always shown (37) | darktable's views; a Grid layout |
| Side panel tabs | Photos, Base, Colour, Correct, Effect and History: darktable's scene-referred groups between Apertrust's own two (38) | A quick access panel; an active-modules tab |
| Window frame and menus | One top bar from the window's top edge; the Mac's native menu bar through muda; one menu button, ☰, on Windows and Linux (39) | The system's title bar above the top bar; muda's Win32 menu bar |
| Popups | Native menus on the Mac for plain lists and context menus, drawn everywhere else, through one popup service (40) | Win32 popups; egui's `Popup` |
| Dialogs and alerts | Every dialog a native window of its own, modal, sheets on the Mac, Settings modeless; the system's alerts, Linux's drawn by the host; Wayland's parent set, the app keeping modality; a border and drop shadow drawn by the host on Wayland (41); on Linux shown before X11's modal hints; framed by the host on Windows and Linux, no icon; Export on every platform (69) | egui's viewports; egui modals |
| Numbers | Shown, typed and stored as darktable does: a value read-only at rest that becomes a field on a click (42) | A value field always shown |
| Settings and folders | One schema, `data/settings.schema.toml`, generating typed statics per owner; `apertrust-base`; the user data folder (43) | Settings declared in Rust; XDG's split by kind |
| Diagnostics | tracing with darktable-style `-d` domains, short lines and a startup banner; puffin for frames (44) | Tracy, a C++ client |
| Data files | A top-level `data/`, each file embedded at build time by the crate that parses it (45) | Files read from an installed folder |
| Library | One photo from a terminal, folder mode first, the Library and the Project; nested albums and smart albums; Open With into folder mode (46) | A database as the truth |
| Edits and sidecars | `.apxmp` and the optional `.apdata` beside the photos, or in a library's folder; version 2 holds the history as a log of steps (47, 48, 49) | A central folder of edits; history in memory only |
| Metadata | Apertrust's own XMP layer on quick-xml, its target conformance to ISO 16684-1:2019, held by its fixtures; EXIF read by rawler (50) | exiv2, exiftool-rs |
| darktable edits | Imported once every module they use is ported; ratings, labels and keywords read either way (51) | A partial import |
| Several computers | Two copies of a sidecar compared by clock vectors; diverged edits kept as a second virtual copy; metadata combined field by field; no locks (52) | Keeping one side by modified time; locks |
| Finding folders and photos | Roots by an id file; photos by root, path and fingerprint; listing in bulk; safe writes; a queue per root (53) | Volume UUIDs or absolute paths; a whole-file hash |
| Library API | Four layers, each calling only the one below, over a storage interface for local files and the server; clients see only `LibraryCommand` and `LibraryEvent` (54) | An SQL API; a second API beside the protocol |
| Photos tab | Browse pinned, one name in every mode, its top line saying what is open; then Filters, Keywords, Metadata and Styles; Filters over the open folder's index in folder mode from Phase 3; one Keywords module, folder mode's keyword list in the user data folder; Metadata the file's facts and darktable's fields in one (73) | A Library tab and module; filters only with the library's cache; a keyword list module of its own |
| Library cache | One per library per machine, rebuilt from the files, its engine chosen by spike L2 (55) | A database in the library's folder |
| Cache | One cache folder by kind, holding only what can be rebuilt with nothing lost: each entry one file, the file system its index, last use its modified time; thumbnails and later previews within one limit, `cache.limit`, evicted oldest first in the background, or unlimited by the person's choice (`cache.limited`); nothing removed by age, library caches and local copies kept until the person removes them, those long unopened offered for removal (`cache.unused_after`, Never a choice); sizes, the limit and Clear on Settings › Storage and in `apertrust-cli cache`; root maps, pending changes and conflict copies in the user data folder ([cache.md](../cache.md); 76) | A per-kind index of entries; unlimited by default, as darktable's |
| Server and sync | A self-hosted server, planned: a container on a NAS that the app optionally connects to, the originals and sidecars in its folders, the desktop editing local copies and sending back sidecars, merged by clock vectors; server roots; one server, `apertrust-cli serve` (71) | A NAS share alone; a database on the server as the truth; locks |
| Export | A job of the engine's session; darktable's sizes, resample, formats, variables and disk storage; `apertrust-export` with no GPU (56) | A job thread or a second device; writing in place |
| Jobs | darktable's control jobs, in `apertrust-base`'s `jobs` module (57); each job reporting its whole row in its own events, folded by a client with no code for any kind, Try again a command the job offers (72) | A crate of its own; GPU jobs as state machines; a client's code for each kind of job |
| Clients | The engine always native; every client through the session protocol; browsers and tablets remote; one server, `apertrust-cli serve`, for sync and later the engine (58, 71) | In-browser processing |
| Events | One event system: each event a fact or a notice, numbered against commands, sent through one outbox, taken in by one reducer per client (59) | A global bus of signals |
| AI inference | ONNX Runtime, the WebGPU execution provider by default, vendor providers as optional packs; copies between devices for now (60) | Burn on wgpu; bundled vendor providers |
| Agents | MCP through rmcp, a thin client of the session protocol; assistants a core capability, not the headline (1, 58) | MCP with its own logic |
| Correctness | During the migration, ΔE2000 golden tests against darktable-cli (61) and a whole-pipeline exit test (62); after it, Apertrust's own renders as baselines (2) | Visual review |
| Licence | GPL-3.0-or-later; darktable's copyright and provenance kept in ported files; bundled fonts under the OFL, icons derived from Lucide under ISC and MIT, with their notices (64) | A permissive clean-room licence |
| Names and crates | Apertrust, its parts prefixed `apertrust-`; few crates (65) | — |
| Decision records | One file per decision, with its options and reasons (66) | — |
| Repository | `github.com/apertrust/apertrust` and apertrust.org; every commit signed off (DCO); AI help disclosed once, in the README and the AI policy; the agent kit in `apertrust/agent-kit`, the hooks in the main repository (67) | An `Assisted-by:` trailer on every commit |
| CI | Every pull request: lint, cargo-deny and the tests on Linux (lavapipe), clippy on Windows and macOS; Windows' tests when a listed Windows path changes, Windows' and macOS's with the `full-ci` label; every platform's tests each night, a failure opening an issue; a guard keeping the path list current (70) | Every platform's tests on every pull request; building only; paid GPU runners |

## Foundations and principles

| Number | Decision |
| --- | --- |
| [0001](0001-what-apertrust-is-the-principles.md) | What Apertrust is: the principles |
| [0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md) | Rewrite darktable first, then Apertrust's own way |
| [0003](0003-the-reference-is-darktables-latest-stable-release.md) | The reference is darktable's latest stable release |
| [0074](0074-masks-ported-from-darktables-master.md) | Masks ported from darktable's master |
| [0004](0004-modules-for-users-operations-for-algorithms.md) | Modules for users, operations for algorithms |

## Language and platforms

| Number | Decision |
| --- | --- |
| [0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md) | Rust, and no C or C++ unless no Rust option exists |
| [0006](0006-two-programs-on-three-desktop-platforms.md) | Two programs on three desktop platforms |
| [0007](0007-a-software-gpu-instead-of-a-cpu-path.md) | A software GPU instead of a CPU path |
| [0068](0068-packages-built-by-cargo-packager.md) | Packages built by cargo-packager |

## GPU and kernels

| Number | Decision |
| --- | --- |
| [0008](0008-a-gpu-resident-fp32-pipeline-on-wgpu.md) | A GPU-resident fp32 pipeline on wgpu |
| [0009](0009-slang-for-every-kernel-compiled-to-wgsl.md) | Slang for every kernel, compiled to WGSL |
| [0010](0010-fxc-on-direct3d-12-and-no-writes-at-run-time-indices.md) | FXC on Direct3D 12, and no writes at run-time indices |
| [0011](0011-a-differentiable-pipeline-and-matching-a-reference-photo.md) | A differentiable pipeline, and matching a reference photo |

## Engine

| Number | Decision |
| --- | --- |
| [0012](0012-planning-regions-scale-and-cache-points.md) | Planning: regions, scale and cache points |
| [0013](0013-tiled-exports-equal-to-untiled-ones.md) | Tiled exports equal to untiled ones |
| [0014](0014-row-strips-for-images-over-one-binding.md) | Row strips for images over one binding |
| [0015](0015-whole-image-passes.md) | Whole-image passes |
| [0016](0016-geometry-for-operations-that-move-pixels.md) | Geometry for operations that move pixels |
| [0017](0017-threads-and-the-engines-session.md) | Threads and the engine's session |
| [0018](0018-one-submit-gate-for-the-engine-and-the-window.md) | One submit gate for the engine and the window |
| [0019](0019-scopes-and-readouts-counted-on-the-gpu.md) | Scopes and readouts counted on the GPU |
| [0077](0077-a-photo-opens-in-stages.md) | A photo opens in stages |
| [0078](0078-undo-redo-and-the-edits-history.md) | Undo, redo and the edit's history |
| [0079](0079-colour-pickers-one-api-for-modules-and-the-colour-picker.md) | Colour pickers, one API for modules and the Colour picker |

## Operations and the plugin interface

| Number | Decision |
| --- | --- |
| [0020](0020-operations-as-native-plugins-one-crate-each.md) | Operations as native plugins, one crate each |
| [0021](0021-the-operation-interface.md) | The operation interface |
| [0022](0022-pipeline-order-and-the-default-pipeline.md) | Pipeline order and the default pipeline |

## Raw stage

| Number | Decision |
| --- | --- |
| [0023](0023-rawler-its-camera-data-fixed-upstream.md) | rawler, its camera data fixed upstream |
| [0024](0024-the-cameras-data-in-the-image-context.md) | The camera's data in the image context |
| [0025](0025-lens-correction.md) | Lens correction |

## Colour

| Number | Decision |
| --- | --- |
| [0026](0026-colour-profiles-and-colour-management.md) | Colour profiles and colour management |
| [0027](0027-both-of-darktables-white-balances.md) | Both of darktable's white balances |

## Modules and bindings

| Number | Decision |
| --- | --- |
| [0028](0028-modules-bound-to-operations.md) | Modules bound to operations |
| [0029](0029-a-module-is-darktables-page.md) | A module is darktable's page |
| [0030](0030-controls-as-darktables.md) | Controls as darktable's |
| [0031](0031-views-and-curves-of-the-same-stored-values.md) | Views and curves of the same stored values |
| [0032](0032-modules-off-until-first-touched.md) | Modules off until first touched |
| [0033](0033-one-tone-mapper-per-edit.md) | One tone mapper per edit |
| [0034](0034-layers-masked-module-instances-in-a-fixed-order.md) | Layers: masked module instances in a fixed order |

## Desktop UI

| Number | Decision |
| --- | --- |
| [0035](0035-egui-for-the-desktop-and-the-browser.md) | egui, for the desktop and the browser |
| [0036](0036-the-design-system.md) | The design system |
| [0037](0037-one-view-the-photos-tab.md) | One view, the Photos tab |
| [0038](0038-side-panel-tabs-as-darktables-groups.md) | Side panel tabs as darktable's groups |
| [0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md) | One top bar, native menus on the Mac, one menu button elsewhere |
| [0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md) | Popup menus: native on the Mac, drawn elsewhere |
| [0041](0041-dialogs-as-native-modal-windows.md) | Dialogs as native modal windows |
| [0069](0069-dialogs-on-windows-and-linux-framed-by-the-host.md) | Dialogs on Windows and Linux, framed by the host |
| [0042](0042-numbers-shown-typed-and-stored.md) | Numbers shown, typed and stored |
| [0075](0075-the-slider-marker-a-setting.md) | The slider's marker, a setting |

## Settings, folders and diagnostics

| Number | Decision |
| --- | --- |
| [0043](0043-settings-and-apertrusts-folders.md) | Settings and Apertrust's folders |
| [0044](0044-logging-the-banner-and-versions.md) | Logging, the banner and versions |
| [0045](0045-a-top-level-data-folder.md) | A top-level data folder |
| [0076](0076-the-cache-kinds-limits-and-control.md) | The cache: kinds, limits and control |

## Library and sidecars

| Number | Decision |
| --- | --- |
| [0046](0046-sidecars-are-the-truth-folder-mode-library-and-project.md) | Sidecars are the truth: folder mode, Library and Project |
| [0047](0047-where-edits-live-and-what-the-sidecar-holds.md) | Where edits live, and what the sidecar holds |
| [0048](0048-the-apxmp-format.md) | The .apxmp format |
| [0049](0049-the-apdata-file-as-darktables-dtdata.md) | The .apdata file as darktable's .dtdata |
| [0050](0050-our-own-xmp-layer.md) | Our own XMP layer |
| [0051](0051-importing-darktables-edits.md) | Importing darktable's edits |
| [0052](0052-comparing-two-copies-of-a-sidecar.md) | Comparing two copies of a sidecar |
| [0053](0053-finding-folders-and-photos.md) | Finding folders and photos |
| [0054](0054-four-layers-for-the-librarys-code.md) | Four layers for the library's code |
| [0055](0055-the-librarys-cache.md) | The library's cache |
| [0071](0071-a-self-hosted-server-with-sync.md) | A self-hosted server with sync |
| [0073](0073-the-photos-tabs-modules-browse-first-filters-in-folder-mode.md) | The Photos tab's modules: Browse first, filters in folder mode |

## Export and jobs

| Number | Decision |
| --- | --- |
| [0056](0056-export-as-an-engine-job.md) | Export as an engine job |
| [0057](0057-jobs-as-darktables-control-jobs.md) | Jobs as darktable's control jobs |
| [0072](0072-each-job-reports-its-own-row.md) | Each job reports its own row |

## Session protocol, events and AI

| Number | Decision |
| --- | --- |
| [0058](0058-a-native-engine-every-client-through-the-session-protocol.md) | A native engine, every client through the session protocol |
| [0059](0059-one-event-system.md) | One event system |
| [0060](0060-ai-inference-with-onnx-runtime.md) | AI inference with ONNX Runtime |

## Correctness and the golden harness

| Number | Decision |
| --- | --- |
| [0061](0061-golden-image-tests-against-darktable-cli.md) | Golden-image tests against darktable-cli |
| [0062](0062-the-whole-pipeline-comparison.md) | The whole-pipeline comparison |
| [0063](0063-departures-from-darktable-and-views-below-full-scale.md) | Departures from darktable, and views below full scale |

## Licences and the project

| Number | Decision |
| --- | --- |
| [0064](0064-licences-gplv3-darktables-provenance-and-bundled-fonts.md) | Licences: GPLv3, darktable's provenance and bundled fonts |
| [0065](0065-names-and-crates.md) | Names and crates |
| [0066](0066-one-record-for-each-decision.md) | One record for each decision |
| [0067](0067-going-public.md) | Going public |
| [0070](0070-ci-in-three-layers.md) | CI in three layers |
