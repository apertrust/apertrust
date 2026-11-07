# 0017. Threads and the engine's session

## Context

How are the desktop app's threads organised, how does a frame reach the window, and how does any client, the app, the CLI or a server, drive the engine? The UI must never wait on a render, the engine must never depend on UI code, and wgpu holds the device's lock for as long as `poll` waits, so one long wait on the GPU freezes every present.

## Decision

**Threads:**

- The **UI thread** runs egui and presents.
- The **engine thread** owns the pipeline's state and GPU submission, as a **session** (`apertrust-engine::session`).
- **rayon** for CPU work such as encoding; **tokio** only for I/O and the server.
- Channels between them. The UI thread waits only for its turn at the submit gate and a surface reconfigure's own wait ([0018](0018-one-submit-gate-for-the-engine-and-the-window.md)).

**The session:** `Session::start` takes the GPU, the operations, the modules, the engine's budget and `wake`, and yields events without waiting.

- Commands are numbered as they are sent. The thread takes every command that has arrived and renders the latest state once, so a 30-step drag sent at once renders fewer than 30 times.
- Each values report says how many commands it includes (`through`), so the app shows a value it set until a report includes it and a slider never jumps back to one reported late.
- `wake` is the only UI hook the engine has, the app's `request_repaint`; the CLI and a server drive the same session.
- The binaries hand the session the operations and modules (`apertrust_operations::OPERATIONS`, `apertrust_modules::MODULES`), so the engine depends on no operation crate.
- Events go out through one outbox ([0059](0059-one-event-system.md)).

**Frames:** each frame is copied into whichever of two display storage buffers was not published last; a published buffer is never written while it is the latest. The app's paint callback draws one triangle over egui's viewport, each fragment reading its own pixel by window position, so nothing is sampled, decoding sRGB first where the window's format encodes itself. The copy costs one pass and two frame-sized buffers (2 × 61 MB at 2400×1600).

**The engine thread never waits on the GPU:** it polls without waiting every 0.25 ms until a work-done callback fires. GPU errors are caught in error scopes and become events, a panic in a batch is reported and drops the photo, and a session whose thread has ended says so once.

**Diagnostics:** tracing for logs and spans ([0044](0044-logging-the-banner-and-versions.md)); puffin for frame profiling.

**Planned, with several photos** ([0037](0037-one-view-the-library-as-a-tab.md)): the live photo's frame first, then one bounded submission of background work, decoding off the engine thread on a job worker ([0057](0057-jobs-as-darktables-control-jobs.md)).

## Options considered

- Rendering on the UI thread, or the session's thread in the app: the UI would wait on renders, and the engine would depend on the app.
- An egui texture for the photo, or an `Rgba32Float` texture: needless copies, and widths a multiple of 16.
- Drawing the pipe's output directly, or rendering into two alternating outputs: plans around two outputs, and buffers a rebuild frees.
- A timed wait on the GPU: trips a wgpu-core assertion when another thread polls.
- Tracy: its client compiles C++ ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).

## Consequences

- On the M4 a 30 MP raw opens in 0.85 s with the window responsive; a session's frame equals a direct render byte for byte.
- The `desktop-architecture` skill holds these rules.
