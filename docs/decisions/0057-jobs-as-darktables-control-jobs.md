# 0057. Jobs as darktable's control jobs

## Context

Exports, a folder's reading, thumbnails, imports and file moves are long work that must run beside editing, be cancellable and show progress. darktable has a proven scheduler for this, its control jobs (`src/control/jobs.c`, `progress.c`, `control_jobs.c`), whose order its users know. Apertrust adds one constraint darktable lacks: GPU work is submitted by the engine thread alone, live frame first ([0017](0017-threads-and-the-engines-session.md)).

## Decision

**Jobs are darktable's control jobs, restated in `apertrust-base`'s `jobs` module** behind a `jobs` feature, std threads and `tracing` only, nothing about images, colour, the GPU or the UI.

**From darktable, at the reference release:**

- its five queues in its order: `UserForeground` (the person's actions), `SystemForeground` (thumbnails), `UserBackground` (a folder's reading), `UserExport` (export, print; one at a time) and `SystemBackground` (crawlers, rescans), and `run_now` for its synchronous jobs;
- its priorities, 4 for the foreground queues and 0 for the rest, the head of strictly highest priority winning, ties to the earlier queue, every other head gaining one each turn;
- `SystemForeground` a stack of 30 deduplicated by kind and key: a running equal discards the new job, a queued one moves to the top, the bottom is discarded past 30;
- its states, initialized, queued, running, finished, cancelled, discarded;
- its progress, made as the job is added so it shows while queued: how far it has got (darktable's fraction, as done and total, [0072](0072-each-job-reports-its-own-row.md)), a message, cancellable, reported at most every 0.5 s;
- its cancel: a queued job never runs, a running one stops at its next check;
- its workers: 7 where the machine has at least 4 threads, else 4; and its `-d control` lines.

The scheduling rules are pure functions (`jobs::schedule`'s `add` and `pick`), held by tables worked through darktable's code.

**The API:** `Control` (`start`, `add`, `run_now`, `cancel`, `state`, `pending`, `shutdown`), `Job` (`new(kind, description, body)`, `dedup`, `progress`), `Context` for the body (`cancelled`, what it reports, and `wait`, which waits for another thread's answer and returns at cancel); a body returns its failure, and a panic is caught as one. Messages are ids with arguments the client translates. `JobEvent`s report jobs with a progress, everything a job's row shows (planned, [0072](0072-each-job-reports-its-own-row.md): `Added`, `Progress`, `Title`, `Detail`, `Destination`, `State`, `Ended`); what a job produced for a script, such as an export's files, goes in its caller's own events.

**The engine's part:** each session owns one `Control`. A job's body hands the engine thread `GpuTask`s and waits for them; the engine thread takes the commands, renders the live frame, then steps one waiting task by the same order and ageing, one submission at a time, a cancelled task dropped before its next step. Jobs reach clients as `Event::Job` (`Added` naming the command that started it) and are cancelled by `Command::CancelJob { job }`, refused for a job that cannot be cancelled; a failed job that offers it is tried again by `Command::RetryJob { job }` (planned, [0072](0072-each-job-reports-its-own-row.md)). The session's drop steps tasks until the control has shut down. The edit writer stays a thread of the session, so a save never waits for a worker.

**Callers:** export `UserExport` ([0056](0056-export-as-an-engine-job.md)) and its preview `UserForeground` with no progress; a folder's reading `UserBackground`, cancellable; thumbnails and stills `SystemForeground`, deduplicated by photo, size and edit; the thumbnail crawler `SystemBackground`, paused by an export; move, copy, rename and Import… `UserForeground`, cancellable, delete not cancellable; rescans and conflict copies `SystemBackground`; Rebuild Cache `UserBackground`; Apply edit, styles and virtual copies `UserForeground`; later print, AI batches and tethering.

**Shown:** one spinner at the top bar's end beside Saved while any job with a progress is queued or running, the warning sign in its place after a failure until it is dismissed or tried again, nothing otherwise ([0072](0072-each-job-reports-its-own-row.md)); a click opens the jobs window, a modeless window of the dialog host ([0041](0041-dialogs-as-native-modal-windows.md)), one row per job, each part from the job's own events ([0072](0072-each-job-reports-its-own-row.md)). A job the person started opens it, a system job never does, a thumbnail's never shows, and closing it stops no job. No progress on the Dock, the taskbar or a launcher.

**Where it differs from darktable, each for a reason:** a job's GPU work is stepped by the engine thread, not rendered on its worker; no reserved workers; plain events, not a proxy into the toolkit; message ids, not translated text; the worker count without darktable's memory term, since jobs keep pixels on the GPU under the session's budget; no kicker thread; the throttle for every caller; no progress without a job; quitting asks first, cancellable jobs stopping and the rest finishing; a failure is a failure, not a cancel; the spinner and window in place of darktable's background jobs panel, its rows told by the jobs themselves ([0072](0072-each-job-reports-its-own-row.md)).

## Options considered

- A crate of its own for jobs: a small module of what every program sets up.
- The API in the engine: the library may not depend on the engine.
- GPU jobs as state machines on the engine thread: unlike darktable's, and a second mechanism for the library.
- Workers submitting to the GPU themselves: the live frame would wait behind them.
- rayon for job bodies: they block on disks and shares.

## Consequences

- Every long piece of work is a job on darktable's queue for its kind; none starts a thread of its own.
- Stills for several photos on screen ([0037](0037-one-view-the-library-as-a-tab.md)) take the same background turn.
