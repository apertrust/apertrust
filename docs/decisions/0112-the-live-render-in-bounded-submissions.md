# 0112. The live render in bounded submissions

## Context

The window and the engine share one wgpu device and its one queue ([0017](0017-threads-and-the-engines-session.md)), and a queue runs its work in order. A live frame is one submission ([0018](0018-one-submit-gate-for-the-engine-and-the-window.md)), so while a slow render runs the window's own frames wait behind it: egui-wgpu's `get_current_texture` blocks on the UI thread until the frames queued behind the render have run, and the window, a slider's knob with it, moves at the render's rate. A Colour equalizer drag with its guided filter renders for 78 ms on the Radeon 780M, 72 ms on the M4 and 39 ms on the RTX 5060 at 1424×1899.

A second cause sits beside it: `Gated` holds the submit gate round every eframe callback, the acquire's wait for its surface image included, both behind queued GPU work and for vsync. So each submission of the engine's waits up to a display frame for the gate, longer during a resize, whose reconfigure holds the gate while the GPU finishes the work already submitted.

darktable stays smooth because its interface never touches the compute device's queue: GTK draws on the CPU and its pipes run on worker threads. The editors that, as Apertrust, share their window's queue keep the work ahead of each window frame short: Greycard bounds its background work to 8 ms submissions, half a frame at 60 Hz, and CalibRaw cuts its workers' renders into chunks by measured cost. How does the live render stop holding the window back, at 60 Hz, with the same frame on every computer?

## Decision

**A, built at Fit, planned in the zoomed view: the live render in bounded submissions.** The fitted viewport's render and the detail pipeline's are cut into chunks between passes, each its own submission through `GpuContext::submit`, at most two in flight. The frame is bit-identical to the one submission's.

- **Chunks from the last render's times.** Each chunk takes passes in order while the sum of their GPU times, measured by timestamps in the last render of the same plan, stays within the bound. A pass with no time yet is a chunk of its own: a new plan's first render, a pass not run before, a device without timestamp queries. A pass longer than the bound is a chunk of its own until row strips split it (below). A render within the bound stays one submission, as an exposure change on the RTX 5060 is. The planner is a pure function of the times and the bound.
- **The bound,** `CHUNK_BOUND`: about 12 ms of GPU time now, one chunk per window frame under today's gate. About 4 ms once the gate shrinks or goes (below).
- **Two in flight.** Chunk i + 2 is submitted once chunk i's work is done (`on_submitted_work_done`, polled without waiting every 0.25 ms, [0017](0017-threads-and-the-engines-session.md)). The GPU is to stay busy between chunks, a target rather than a guarantee: chunks shorter than the poll can both finish before it. A window frame waits for at most the chunk under way and the one after it, plus any turn of its own queued before them: a job's step, the scopes' count, a sample-only render (decision 79).
- **The last chunk** holds the copy into the display buffer and the timestamps' resolve and copy. A picker's probe and a readout's pass stay in the chunk of their operation's last pass, their copies into staging with them. The frame is published once the last chunk is done. A sample-only render ([0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)) stays a turn of its own, short and not chunked, outside the two in flight. Each chunk settles the cache points, the edit cache and the scratch's results it wrote at its end, so their keys always match their buffers.
- **No command between a frame's chunks.** Commands that arrive while a frame's chunks run wait in the channel and are taken, coalesced, at the engine thread's next turn: a later command's `write_buffer` of its uniforms would reach the chunks still to submit, mixing two edits in one frame.
- **A render already stale when a new value arrives is finished, not dropped.** The newest value wins at the next render ([0017](0017-threads-and-the-engines-session.md)'s coalescing).
- **The zoomed view** ([0099](0099-the-zoomed-view-a-detail-pipeline-beside-the-fitted-one.md)): one frame is a sequence of bounded turns, published with its last. The detail's chunks come first and the fitted frame's follow in a frame of their own. Where decision 99 joins the two in one encoder, the fitted passes' chunks come first and the detail's after them, one frame. A whole-image round's tiles stay a submission each.
- **Background work** ([0057](0057-jobs-as-darktables-control-jobs.md)): a job's step comes only after the live frame's last chunk, one step on the GPU at a time, each step within spike X1's bound, unchanged.
- **The cost target:** a chunked frame's time from its command at most 3% over the one submission's. Measured: one submission's round trip (recording, `queue.submit`, the 0.25 ms poll and reading the timestamps) costs 0.45 ms on the Radeon 780M (8.79 ms against 8.33 ms of GPU time) and 0.35 ms on the RTX 5060 (1.93 against 1.56 ms), the X-T5's 40 MP raw at 2400×1600, 30 exposure changes. Two in flight are to hide most of it behind the GPU.

**The gate's second cause, planned with wgpu 31.** wgpu 30 fails a reconfigure while another thread submits, which is why the gate exists. wgpu's trunk no longer does: [#10296](https://github.com/gfx-rs/wgpu/pull/10296) makes `configure` unconfigure first and wait for the queue with no failure, and [#10675](https://github.com/gfx-rs/wgpu/pull/10675) holds the queue's submission lock round the HAL's configure, so submissions on other threads wait for it. Once wgpu 31 and an egui-wgpu on it are out and a stress test of resizes during renders passes on the M4, Linux and Windows, the gate shrinks to a reconfigure's own span, or goes, and the bound drops to about 4 ms.

**What the UI thread waits for:** its turn at the gate; a reconfigure's wait for everything submitted; and the acquire of its surface image, which waits for the GPU work queued before its earlier frames, the engine's included. Today that is up to a whole render; with A, at most two chunks, plus any turn of its own queued before them: a job's step, the scopes' count, a sample-only render (decision 79).

**The target:** with A under today's gate, a window frame time under about 20 ms (Done for A, below); 60 Hz once the gate shrinks with wgpu 31 and the chunks drop to about 4 ms. 120 Hz needs chunks of about 2 ms: planned later.

**Scope.** The viewport at Fit first, then the zoomed view. **Row strips, planned next:** at 100% and on 61 MP photos a pass is longer than the bound (one full-resolution pass 27 to 29 ms on the M4 and the 780M, highlights' 44 ms on the M4), so such passes, row-local and pointwise, are split into decision [0014](0014-row-strips-for-images-over-one-binding.md)'s row strips, as an export's steps are (`Pipeline::parts`, `export_job::STRIP_BINDING`), each strip a chunk.

**Measurement, before A and after it:**

- in the app under `-d perf`, a line once a second while a gesture or a render is under way: the window's frame interval (median, 95th percentile, worst and how many passed 20 ms), its CPU time (eframe's `frame.info().cpu_usage`) and the rest of the frame, the interval less the CPU time: the acquire, the present, vsync's pacing and any idle time, not a wait (the gate's wait is measured in the engine, next);
- in the engine under `-d perf`, the time `GpuContext::submit` waits for the gate, logged above 0.5 ms, and per render its submissions, their summed gate wait, the GPU time and the wall time;
- checks that need no code: Mesa's overlay layer with `acquire_timing` on Linux, and the Metal HUD (`MTL_HUD_ENABLED=1`) on macOS.

**Done for A:** the window's frame time stays under about 20 ms during a render of 100 ms or more, on the Radeon 780M, the M4 and the RTX 5060. **A hand check before A is built:** whether the whole desktop stutters on the 780M during a long render, the GPU's hardware queue being shared by every app; a bounded chunk would help there too.

**B, planned after A: a lower resolution while a slider moves.** Only where a drag's measured frame time passes a budget, the fitted pipeline renders the drag at a lower scale, chosen as the drag starts and fixed until it ends, and the release (`done: true`) renders it sharp, as decision 99's `detail_pending` does for a zoomed drag. It is a lower scale of the fitted pipeline itself, not a third pipe. The budget, the scale, and what the scopes and the pickers read during such a drag are settled when B is designed.

## Options considered

- **A small preview pipe, as darktable's:** rejected by decision 99, a third size whose values would differ from the fitted view's; and it does not bound the window's wait.
- **Decoupling the window's drawing from the queue:** egui and wgpu offer no way to draw the window elsewhere. Present modes change how many frames queue, not that they queue behind the render, and drawing the interface on the CPU needs a host copy of the photo every frame.
- **A second queue or device:** wgpu has one queue per device. A second device on the same adapter is still the graphics ring with RADV, so on the 780M nothing runs in parallel without a compute-only queue family, reached only through `unsafe` hal code; and the frame would cross by a host copy each frame or per-backend interop.
- **B alone:** it helps drags only, and is unbounded: an expensive operation still holds the window for its whole render.
- **Dropping a stale render** at a chunk boundary, as darktable's pipe stops at the next module: it can starve a continuous drag, the photo updating only when the pointer pauses.
- **Chunks of about 4 ms under today's gate:** each chunk could wait a display frame for the gate, starving the GPU.
- **Driving the main window's painter in the app now,** the gate held round `configure` alone: egui-wgpu configures inside its paint, so it needs a painter of the app's own or a fork, and it is the gating of selected callbacks decision 18 found fragile.
- **One chunk in flight:** the GPU idles for each round trip, 0.35 to 0.45 ms a chunk. **A fixed number of passes a chunk:** a full-resolution pass and a viewport one differ by twenty times.

## Consequences

- Decision 18's frame of one turn becomes a sequence of bounded turns, and its account of what the UI thread waits for gains the acquire; decision 17's does too. Decisions 99 and 57 order the detail's chunks and the job steps after them.
- Tests: the chunked frame equal to the one submission's byte for byte on the fitted, detail and tiled viewport pipes; the planner's chunks within the bound but for a single pass, a pass with no time alone, the last chunk holding the copy and the timestamps; the turns' order; a command sent during a frame's chunks rendering only in the next frame. The golden cases do not move.
- The plan (roadmap, Phase 2, Smooth dragging): the measurement and the hand check, A at Fit, A in the zoomed view, row strips, the gate after wgpu 31, then B; 120 Hz later.
- The photo's own rate is each operation's speed, unchanged by A: the colour equalizer's guided filter (#702) and fusion in Phase 2.
