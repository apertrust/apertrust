# 0072. Each job reports its own row

## Context

The jobs window ([0057](0057-jobs-as-darktables-control-jobs.md)) builds each row from four places: the job's events (`JobEvent`: `Added` with its kind, queue, message and whether it can be cancelled, `Progress` with a fraction, `Message`, `State`, `Ended` with a state and a failure), the export's `Event::Export` for each photo, the export request the app remembers sending, and the app's clock. Only an export has detail lines, through export code in the app's mirror, and Try again is the app rebuilding the export it sent. Every other client, the CLI, the server and MCP, would repeat that code for each kind of job.

darktable's background jobs panel (`libs/backgroundjobs.c` at the reference release) shows what `control/progress.c`'s proxy gives it: a message, a bar filled by a fraction, and a cancel button. The rest of a row is Apertrust's shell, which decision 2 does not mirror.

## Decision

**Each job reports everything its row shows in its own events, and a client's mirror folds them into the row with no code for any kind of job.** Planned, built in the steps of the roadmap's jobs window ([roadmap.md](../roadmap.md#the-jobs-window-and-the-job-api)).

**The events,** base's `JobEvent`, which the session sends as `Event::Job` ([events.md](../events.md#a-jobs-events)):

| Event | Fields | Kind | darktable's |
| --- | --- | --- | --- |
| `Added` | `job`, `kind`, `queue`, `title: Message`, `cancellable`, `destination: Option<Destination>`, `detail: Option<Message>`, `progress: Progress`, `retries: Option<JobId>` | notice | `added`, `cancellable` |
| `Progress` | `job`, `done: u64`, `total: Option<u64>` | fact, by job | `updated`, its fraction `done / total` |
| `Title` | `job`, `title: Message` | fact, by job | `message_updated` |
| `Detail` | `job`, `detail: Option<Message>` | fact, by job | none |
| `Destination` | `job`, `destination: Destination` | fact, by job | none |
| `State` | `job`, `state` | fact, by job | the job's state |
| `Ended` | `job`, `state`, `outcome: Option<Message>`, `failure: Option<Failure>`, `at: SystemTime`, `offers: Vec<Offer>` | notice | `destroyed` |

- **`Progress`** is done and total in place of a fraction: the bar fills at `done / total`, `done` clamped to `total`, and is indeterminate with no total. `Added` carries the first, `done` 0 and the total where the job knows it.
- **`Title`** is the row's title, darktable's message ("Exporting 3 / 12 to file on disk"); **`Detail`** the one line under the bar ("3 of 12 done · now IMG_2044.ARW", "Next in line: 5 photos, to …"), `None` clearing it.
- **`Destination`** is `Folder(PathBuf)` or `File(PathBuf)`: where the job's results go, the row's "To …" line (a file's folder) and what Show in Finder reveals, a file selected in its folder.
- **`Ended`** carries the `outcome`, the ended row's title ("10 of 12 exported"); the `failure` that makes the job a failed one; `at`, the wall clock as the control ends the job; and the `offers`, what the job can do once ended.
- **`Failure`** is `{ cause: String, item: Option<PathBuf>, file: Option<PathBuf>, reason: String }`: a code the client words with Fluent (`job-cause-<cause>`), the item it failed on (a photo), the file it could not write, and the error in plain words for the log and for a client with no words for the code. A panic is cause `panicked`, its text the reason. A job ends failed exactly when its `failure` is set.
- **`Offer`** is `Retry { items }`, the items a retry would take. Showing a destination is not an offer: Show in Finder comes with the destination, while the job runs too.
- **`retries`**, on `Added`, names the ended job this one tries again, so a client puts the new row in its place.
- **Messages stay ids with arguments,** worded by the client with Fluent ([0036](0036-the-design-system.md)), never English from the engine.

**A job's body,** base's API:

- `Job::new(kind, description, body)`, `.progress(title, cancellable)` as now, and `.destination(destination)`, `.detail(message)`, `.total(total)` and `.retries(job)`, which `Added` carries.
- `Context`: `set_progress(done, total)`, `set_title`, `set_detail`, `set_destination`, `set_outcome` and `offer`, beside `cancelled` and `wait`.
- The body returns `Result<(), Failure>`, a panic caught as `panicked`.
- **The throttle stays darktable's,** `PROGRESS_UPDATE_INTERVAL` of `control_jobs.c`: each of `Progress`, `Title`, `Detail` and `Destination` goes out at most every 0.5 s, the first and the last always. The outcome and the offers are held until `Ended`.

**Try again is `Command::RetryJob { job }`,** answered by the session:

- A job that can be tried again offers `Offer::Retry` and, before its body returns, leaves the session what the retry runs: for an export, `Command::Export` of the photos that failed and those not reached, with the same settings and places.
- The session keeps the latest 64 for its life. `RetryJob` runs the kept command as if it were sent then, so an export takes each photo's edit as it is at that moment (decision 56's D12). Its job's `Added` names `retries: Some(job)`, and its `command` is the `RetryJob`'s number.
- A retry is used once. One not kept is refused, `SessionError::NotRetryable { job }`.

**The export reports itself** ([engine.md](../engine.md#the-job)):

- `Added`: the title darktable's per-photo message at 0 ("Exporting 0 / 5 to file on disk"), the total its photos, the destination the path's folder where it holds no variable, and the detail `job-export-queued` with the count and the folder as typed.
- For each photo: its `Title`, a `Detail` naming it with the counts, and `Progress` with the photos done.
- Its `Destination`: the first file written, once there is one.
- At its end: the outcome by its counts; the failure that ended the job, or else the first photo's that failed; and `Retry` where a photo failed or was not reached, but not after a cancel, nor when the export was refused.
- **`Event::Export` stays** as the export's record for scripts and the CLI: each photo's start and its file with its size and tiles, then the counts. No client draws a row from it.

**The app's mirror is a fold:** one row per job, each part from one event ([gui.md](../gui.md#the-jobs-window)). Its `sent` requests, export rows and the app's clock go. The window opens itself for a job on `UserForeground` or `UserExport` whose `Added` names this client's command, and for no other.

**The window's look** follows the wireframe: stacked rows, an 8-point accent bar, a ✕ at the bar's end that only cancels, Clear Finished in the footer for finished and cancelled rows, Dismiss on a failed row beside Try again, the spinner only while a job is queued or running, the warning sign after a failure until it is dismissed or tried again, and no tick. A thumbnail's job has no progress and never shows.

**From darktable** the control is kept whole: its queues, states, cancel and throttle, the title as its message, the bar and the cancel button. Apertrust adds, as its shell's: done and total in place of the fraction, the detail, the destination, the outcome, the failure's cause, item and file, the end's time, the offers and Try again, and an icon by kind (decision 57's J18 and J19).

## Options considered

- **The app's per-kind mirror, as built:** every client would repeat each kind's code, and Try again would exist only in the app.
- **The whole row as one fact, sent on every change:** a simpler fold, but each 0.5 s would resend what never changes, and the facts by field already coalesce by topic for a remote client.
- **Try again as an offer carrying the command to send back:** base's events cannot name the engine's commands, and every client would receive and hold the photos' list for a retry it may never ask for.
- **A fraction with a counting message beside it:** two sources for one bar; done and total give both the bar and its words, and an unknown total its own look.
- **Several detail lines:** the row has one line under its bar; more is the title's or the outcome's.

## Consequences

- `apertrust-base`'s `jobs`: `JobEvent`, `Job`, `Context`, `Failure`, `Destination`, `Offer` and `Progress` as above, the throttle over four values; `Context::set_fraction` and `set_message` go.
- `apertrust-engine`: the export reports itself; `Command::RetryJob` and `SessionError::NotRetryable`; the retries kept; `Event::Job` carries the new events into the protocol.
- `apertrust-app`: `jobs::Mirror` folds `Event::Job` alone; Try again sends `RetryJob`, Clear Finished and Dismiss change the mirror.
- `apertrust-ui`: `Job` takes the row's parts; `ExportDetails` and `ExportFailure` go; `ActivityAction` gains `ClearFinished`.
- The CLI keeps reading `Event::Export` for its lines.
- A new job kind shows in every client with no client code, given its Fluent ids and, for its icon, a line in the design system's table.
