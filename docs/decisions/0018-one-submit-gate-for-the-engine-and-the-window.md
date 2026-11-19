# 0018. One submit gate for the engine and the window

## Context

The window and the engine share one wgpu device ([0017](0017-threads-and-the-engines-session.md)), and eframe reconfigures the window's surface on the UI thread as the window resizes. wgpu-core fails a reconfigure when another thread submits while it waits for the queue (`ConfigureSurfaceError::GpuWaitTimeout`), wgpu's default handler turns that into a panic, and eframe reconfigures inside winit's resize callback, which on the Mac cannot unwind. Catching the error does not recover: egui-wgpu has already recorded the new size and the next frame fails validation. How do the engine and the window share the GPU so this cannot happen?

## Decision

**The engine and the window take turns at one submit gate.**

- **The gate**, `SubmitGate` on `GpuContext`: a lock the holding thread may take again, since a toolkit's callbacks nest, with turns given in the order asked, so a waiting thread enters after the hold under way however soon its holder asks again.
- **Every engine submission goes through `GpuContext::submit`**, which holds the gate for `queue.submit` alone. Nothing in the engine calls `queue.submit`.
- **The window:** the app runs eframe on its own winit event loop (`eframe::create_native`, `EventLoop::run_app`) inside `Gated`, which holds the gate round every eframe callback, so every reconfigure, at start, during and after a resize and when a frame finds the surface out of date, runs while the engine cannot submit, on every platform. Dialog windows hosted beside eframe's run inside the same turns.
- Nothing waits for the engine thread while holding the gate except through `SubmitGate::released`, which frees it for the wait and takes it again, after a panic too. `Session`'s drop joins its thread so.
- **A frame is one turn:** the render, its copy into the display buffer and its timestamps in one submission (`Pipeline::render_then`). The scopes' count stays a second, so a failure of theirs never costs the frame.

**What the UI thread waits for:** a submission asked for before its callback, the CPU time of one `queue.submit`; and a reconfigure's own wait, which wgpu-core makes for everything already submitted on a shared device, a few milliseconds behind an edit at viewport size, longer behind a photo's first render.

**Resizes:** the photo's new size goes to the engine only once the window has kept its size for 150 ms; meanwhile the last frame is drawn bilinearly scaled to the size the next will have.

**Errors:** an uncaptured error on the window's device is logged under `gpu`, each kind once and up to 16 kinds, never a panic. A start that fails returns an error, where eframe would only log it.

## Options considered

- Gating only the callbacks that can reconfigure: fragile, since eframe also paints from other callbacks on some platforms, and a missed path aborts the app.
- Holding only the viewport's size during a resize: other submissions still run during a step.
- Catching wgpu's failed reconfigure: does not recover (above).
- A patched wgpu-core that waits until the queue is empty: a fork to carry through every update.
- A second device for the engine: frames would reach the window only through a host copy.
- Submitting the engine's work from the UI thread, or a gate that lets in whoever comes first: the UI waits on renders, or a frame waits seconds.
- A turn for each of a frame's render, copy and timestamps.

## Consequences

- The `desktop-architecture` skill and AGENTS.md's GPU rule hold it.
- Long submissions lengthen a reconfigure's wait, so background work is bounded per submission ([0057](0057-jobs-as-darktables-control-jobs.md)).
- Once a wgpu release no longer fails the reconfigure the gate can go, though the wait stays.
