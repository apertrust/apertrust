# History and undo

Every change to a photo's edit makes steps of its history, one for each module it changed, as darktable's history lists one item per module: a slider's drag, a preset, a reset, an assistant's request. Undo and Redo move through those steps, the History tab lists them, and the sidecar keeps them, so reopening a photo brings Undo back. This document describes the history: what a step is, how the session serves undo and history to its clients, how the sidecar stores them, and how the work is built. Where edits live and the sidecar's other parts are in [library.md](library.md#sidecars); the session's other commands and events in [session-protocol.md](session-protocol.md) and [events.md](events.md).

**Status.** [Decision 78](decisions/0078-undo-redo-and-the-edits-history.md) records the design, with the options considered, on [decision 47](decisions/0047-where-edits-live-and-what-the-sidecar-holds.md)'s log of steps. Built, as step 9.1: the log in the session's memory, each completed change a step of each module it touched, several sharing a batch id that Undo and Redo move whole, `Reset` one step, `Undo` and `Redo` with the undo records, and `Event::History`; as step 9.2: `GoTo` with its undo record, `Step` with its refusals and the gesture taken in, and the app sending the tone equalizer's release of several bands and its reset of the nine as one `Step { source: You }`; as step 9.3: merging consecutive changes by target, darktable's 10 s and 2 s, timed from when each command was sent and each frame published; as step 9.4: Edit › Undo and Redo and their keys in the app, enabled from the mirror's `History` and naming what they would do; as step 8.2: version 2's text layer, `sidecar::Edits`, with its fixture, and a version 1 file read through it as a step of each module, sharing a batch. The engine still writes version 1, holding the edit at the end, and reads it again the same way. Not built: saving the log, the History tab, the eye, Compress and Truncate, and the CLI's history; the app draws the History tab disabled, and `apertrust-cli edit` writes version 1. The rest of the library's step 9 follows ([The plan](#the-plan)).

## What it serves

| Client | What it needs |
| --- | --- |
| The desktop app | Edit › Undo and Redo and their keys, naming the step ("Undo Exposure"); the History tab; Reset in its header as a step |
| `apertrust-cli` | `edit` adding one change, a step of each module it writes; `history` to list, undo, redo, go to a step, compress and truncate |
| The assistant and the MCP server | One undoable step per request, carrying its source and a note on what it did (principle 8; [ai.md](ai.md)) |
| Batch edits | Apply Edit to the selection and styles: a step of each module on each photo, sharing a batch id, one Undo for all (decision 37) |
| Later | Virtual copies and snapshots (the library's steps 11 and 16), several computers (steps 8 and 12), the web client over the session protocol (decision 58) |

Every client of a session, the app, MCP and later the web client, uses the same commands and draws the same `History` fact. The CLI's `edit` and `history` work on the sidecar outside any session, through the same engine functions ([The CLI](#the-cli)).

## The History tab

The wireframe's History frame, which the API serves:

- one line per step, newest first, then the Default lines, then Original at the bottom;
- a step's line names its module, with darktable's label after a • and a layer after a ·, as darktable's history lists each item's module, a batch's steps each their own; a step with a note shows its note; Reset's one step reads Reset;
- a source tag: You, Auto, Style, Assistant or Default;
- a tooltip with each change, before → after, who made it and when (date and time); a step made on another computer also names that machine ("You on Studio Mac · 14 Mar 2026, 14:02"), with the library's step 12, which brings machines' names; until then the tooltip shows who and when alone;
- a click on a step goes to it: the later steps go grey, and the next edit drops them;
- Shift-click on a step opens its module, as darktable's does;
- an eye on a step's line hides its module, or later its layer, from the view only: no step, nothing saved;
- Compress history and Create a style… under the list, Ctrl-click (Cmd-click on the Mac) on Compress history truncating instead, as its tooltip says, after darktable's ("create a minimal history stack which produces the same image / ctrl+click to truncate history to the selected item", `src/libs/history.c:139-142`); Reset in the header, as a step.

**Numbers.** Original is 0 and the steps count from 1, so the step numbered `end` is the last applied, matching `History::end`. The Default lines carry no number.

**The Default lines** are never greyed and never selected; at end 0 only Original is highlighted. A click on one opens its module, as Shift-click on a step does, and they have no eye.

## The log

Each virtual copy of a photo has one log: its steps in order, and its **end**, how many of them apply, darktable's `history_end`. End 0 is Original.

**The edit is folded from the log:** for each module, its entry at its last step among the first `end`, else its reset state, its defaults for the photo and off when it is switched ([decision 32](decisions/0032-modules-off-until-first-touched.md)). Nothing else stores the edit; the session folds it as a photo opens and keeps it in step with each change, and export, thumbnails and the index read the same fold.

The log is the engine's (`apertrust_engine::history`): `Log`, `Step` and the functions that add, fold and merge, built, and that compress, planned, plain data, used by the session and, planned, by `apertrust-cli`. The sidecar's text layer (`apertrust_library::sidecar`, [decision 54](decisions/0054-four-layers-for-the-librarys-code.md)) holds the steps as strings, knowing no module's meaning.

## A step

### Its fields

| Field | Type | What it holds |
| --- | --- | --- |
| `id` | `StepId`, 16 hex digits | Random, 64 bits, so two logs joined step by step (step 14) do not collide; drawn again on a clash within its log |
| `clock` | `Clock` | The hybrid logical clock of its last change: UTC milliseconds, a counter and the machine id (decision 47; `apertrust_library::clock`, built with step 8's first part). The tooltip's when, and from step 12 its machine's name, from a library's `.apertrust/machines/` or, in folder mode, "another computer" |
| `source` | `StepSource` | `You`, `Auto`, `Style`, `Assistant` or `Default`: the tooltip's who and the line's tag |
| `note` | `Option<Note>` | `Note::Message`, Apertrust's own words as a message id with its arguments, worded by the client with Fluent ("Applied from IMG_2041", `history-reset`); or `Note::Text`, an assistant's or script's own words, stored as written and never translated |
| `batch` | `Option<BatchId>`, 16 hex digits | The session's, shared by the steps of one change that touched several modules, and later by those one command made on several photos; `None` for a change of one |
| `change` | `Change` | `Change::Entry`: the one module it touched with its whole stored state after it, its version, its controls set, its marks and whether it is on, as version 1 writes a module, or an unowned operation's parameters likewise while `Set` exists; or `Change::Reset`: an empty entry for each module and operation Reset reset |

- **An empty entry** is the module's reset state: its defaults, or `off` for a switched module (decision 32).
- **Planned:** the ratings, reject, labels, keywords and metadata a step sets, with the library's steps 14 and 15 (decision 47). Their encoding in a step is part of step 14's scope.
- **Room for layers** (Phase 4, [decision 34](decisions/0034-layers-masked-module-instances-in-a-fixed-order.md)): an entry's key, a module id today, gains a layer and an instance, as darktable keys its history by operation and `multi_priority`. A line then names the layer after a ·; nothing else in the step changes.

### Whole entries after, the before derived

A step stores each module's **whole state after** it, never the change alone:

- an entry migrates on its own when its module's version rises ([decision 48](decisions/0048-the-apxmp-format.md)), where a delta could not be read without the steps before it;
- the fold is a lookup of each module's last entry, with no replay;
- Compress keeps whole entries, so nothing it keeps depends on what it dropped;
- a step can be laid on another log as it is, which the session does when it takes a newer file ([Behaviour](#behaviour)).

**The before is derived, never stored:** a step's before, for a module, is that module's entry at the previous step that touched it, else its reset state. Stored, it would be a second truth beside the log. After Compress, a kept entry's before is the reset state, as darktable's compressed history shows it.

The tooltip compares the two, control by control: each control whose value or mark changed, before → after, and the module turned on or off.

### What makes a step

One completed change makes a step of each module it touched: a gesture ending with `done`, a reset, a preset or a `Step`, a control's quick repeats merged into one ([below](#merging-consecutive-changes)). On the edit's own rules (decisions 28 and 42):

| Command | Step |
| --- | --- |
| `SetControl { done: true }`, with the gesture's `done: false` changes before it | One, its module's |
| `ResetControl`, `Keep`, `ApplyPreset` | One, its module's |
| `ResetModule` | One, its module's entry empty |
| `Reset` | One, with an empty entry for every module the edit holds at the end, switched modules going off (decision 32), and the note `history-reset` |
| `Step { source, note, commands }` | One for each module its commands write, in a batch where there are several |
| `Set` (unowned parameters) | None until the next change, where it is a step of its own in that change's batch |
| `CancelGesture` | None ([decision 42](decisions/0042-numbers-shown-typed-and-stored.md)) |
| `Undo`, `Redo`, `GoTo`, `Compress`, `Truncate` | None: they move the end or rewrite the log |

- **A change that changes nothing makes no step,** as it writes nothing.
- **A pending `Set` followed by `Reset`** joins the reset's step, which resets it with the rest.
- **A pending `Set`** with no step after it becomes a step of its own, source You, before an Undo, Redo, GoTo, Compress or Truncate and as the photo closes, so no value waits outside the log.
- **Reset** (the top panel's Reset, Edit › Reset Edit, the History header's ↺) is one step that resets the edit, holding every module it reset, with the same note, on each photo of the selection when several are chosen, as a batch. The log stays; Compress shrinks it.
- **Discard History…** (decision 78, planned with 9.8) is darktable's discard, a command of its own, `DiscardHistory`: it clears the history and resets the edit, doing what going to Original and then Compress does, since Compress at end 0 deletes the whole log, so it makes no step, as one undo record that takes it back while the photo is open. It asks first, with Don't ask again (darktable's `ask_before_discard`). It lives in Edit › Discard History…, the photo's right-click menu and the History section's ⋯ options menu, never as a button in the History header.
- **The app's changes of several commands are one `Step { source: You }`:** the tone equalizer's release of the bands a drag moved, and its reset of the nine bands by a double-click or Shift+Delete, each one step, since each writes the tone equalizer alone ([modules/tone-equalizer.md](modules/tone-equalizer.md), [decision 31](decisions/0031-views-and-curves-of-the-same-stored-values.md)). A `Step` takes in the gesture under way when its commands write that gesture's module and its source and note are the gesture's, so the drag's `done: false` changes and the release are one step ([Commands](#commands)).

### A change of several modules

A step is exactly one module's change, as darktable's history item holds one module (`dt_dev_history_item_t`, `src/develop/develop.h:34`). A completed change that touches several modules, a `Step` whose commands write several, a preset row writing several, and later a style or an assistant's request, makes a step of each, as darktable's style and paste add an item per module (`src/common/styles.c`, `src/common/history.c`):

- **In the order written:** each module and operation in the order its commands first wrote it after the last `Reset`; one no command named, as a module another turned off, after them, in id order (`Change::between`).
- **One batch:** the steps share a `BatchId`, the session's, and the change's source and note, as darktable wraps the items in one undo group (`dt_undo_start_group`, `src/common/styles.c:954`; `src/common/history.c:157`, `:200`, `:229`, `:971`), so Undo and Redo move the batch whole while the History tab lists each module. A change of one module has no batch.
- **Reset is the exception:** one step noted `history-reset`, holding an empty entry for every module and operation it reset, shown as one Reset line. In a `Step`, Reset's step takes the place of the `Reset`, holding what no later command wrote, and the modules written after it follow; that step is noted `history-reset`, not with the `Step`'s note, which the other steps keep.
- **The fold is unchanged:** each module's last entry up to the end, so the batch's last step's fold is the edit the change left, and a step inside it the batch's steps up to there.

### Merging consecutive changes

A slider nudged with the arrow keys or the wheel sends a completed change each time. darktable's undo merges such changes by their target, the widget changed (`_dev_undo_start_record_target`, `src/develop/develop.c:1351-1370`), a slider's move or its reset by a double-click alike, for the widgets wired through `dt_iop_gui_changed` (`src/develop/imageop.c:3997-4006`, `src/bauhaus/bauhaus.c:1844-1857`, `:3386-3394`); a change through a module's own callback has no target and never merges (`develop.c:1359`). A change to the widget changed last starts no new undo record within `darkroom/undo/merge_same_secs` (10 s) of the record's start, nor within `darkroom/undo/review_secs` (2 s) of the pipe finishing the last change, `gui_previous_pipe_time`, set as the pipe ends (`develop.c:885`). Apertrust merges the same way, every control and preset row alike, as decision 78 decided, so one Undo takes back what one darktable undo takes back for such a widget.

A completed change merges into the step before it when all of these hold:

- both have the same **target**, darktable's widget: the control a `SetControl` or `ResetControl` names, the preset row an `ApplyPreset` names, or the picker a pick came from ([pickers.md](pickers.md#what-a-pick-writes)), of the same module; so a reset or a preset on the target changed last merges too, and a `SetControl` gesture counts as one change;
- both have the same source, You or Auto, no note and no batch, the change touches one module, and neither is a `Step` of several commands;
- the step is the log's last and the end, made by the session's previous command on the edit (a pick counting as the `Picker` command that asked for its reading, [pickers.md](pickers.md#what-a-pick-writes)), with no Undo, Redo, GoTo, Compress, Truncate or other command on the edit between;
- the change's first write came less than 10 s after the step's first, and, once the first frame whose `through` includes the step's last change is published, less than 2 s after that publication. Until that frame is published only the 10 s counts.

The merged step keeps its id, its entry takes the new state and its clock becomes the latest. A change merged adds no undo record, so one Undo takes back the merged step whole. The two times are darktable's defaults, constants rather than settings (`MERGE_WITHIN` and `REVIEW` in `session/history.rs`).

- **The times.** A change's first write is timed when the client sent its command, as the session receives each command with that time; a frame's publication when the session publishes it. A test hands both in, so it merges and splits without waiting.
- **What counts as a command between:** any command on the edit that writes no value, Undo, Redo, `GoTo` and `CancelGesture` among them, and any command refused, a `Step` refused as `DefaultSource` or `EmptyStep` too; not `RetrySave`, which changes nothing and leaves a gesture under way. A change that changes nothing, a gesture's or a `Set`'s too, makes no step and counts as none: it sets neither a target nor when the change began.
- **A `Step` of one command** has that command's target, so Auto's single writes, each a `Step { source: Auto }`, merge as the person's do. A `ResetModule`, `Keep`, `Set` or `Reset` has no target: it never merges, and a step holding one is merged into by none.
- **A merged step whose changes come back to where it began** stays, its one entry the state now, as darktable's history item stays after its undo record merged; Undo takes it back as any step.

**Pickers and Auto** write with source Auto and gesture semantics. A picker's pick is a change the session makes itself as the reading arrives, through the module's `pick` ([pickers.md](pickers.md#what-a-pick-writes), decision 79): a drag's readings are `done: false` changes ending with the release's `done: true`, one step, merged as a slider's, its target the picker. Auto sends its writes as `Step { source: Auto }`.

darktable's history items merge further: a module's consecutive changes are one item until the person focuses another module (`_dev_add_history_item_ext`). Apertrust merges steps only as darktable's undo merges changes, so the History tab and Undo always agree.

### Original and the Default lines

- **Original** is end 0: no step applied, the photo as Apertrust shows it untouched, darktable's defaults included ([decision 21](decisions/0021-the-operation-interface.md), decision 32).
- **The Default lines** are the modules every photo runs, those not switched, listed between Original and the first step in darktable's history order for a new photo: those darktable has on by default with no on/off button first (Raw black/white point, Demosaic, Output colour profile), then those on by default with one (White balance, Highlight reconstruction, Orientation), as its `_dev_add_default_modules` adds them (`src/develop/develop.c:2142-2171`, called at `:2339`), then every other, Exposure, Colour calibration and the tone mapper among them, which darktable's workflow applies as presets after them (`_dev_auto_apply_presets`, `:2343`); each group in the pipe's order. They are derived from the photo's defaults and never stored, carry no number and are not places in the log to go to ([The History tab](#the-history-tab)). darktable's Original, its auto-applied modules not yet applied, has no Apertrust counterpart: the modules every photo runs cannot be turned off (decision 32).
- **Planned:** darktable's auto-applied presets (Phase 3) and a project's start adjustments (the library's step 38) are written as real steps with source Default as they are applied, by the session itself; a client cannot send `StepSource::Default`.

## Behaviour

**Undo follows darktable's undo** (`src/libs/history.c:664-686`: every change to the history, a click on an item among them, records the history and its end for Ctrl+Z). The session keeps, in memory, a stack of **undo records**, each what one command on the history changed:

| Record | Undo | Redo |
| --- | --- | --- |
| One change's steps added, one or a batch's | The steps leave the log, as darktable's undo restores the history without them, a batch's whole as darktable's undo group; any steps they dropped after the end come back, and the end as it was | The steps again, the steps after the end dropped again |
| `GoTo` | The end where it was | The end where `GoTo` put it |
| `Compress` | The log as it was before | Compress again |
| `Truncate` | The log as it was before | Truncate again |

A change merged into the step before it adds no record, as darktable's merged change records no undo: Undo takes back the merged step whole.

- **Two stacks:** the done records and the undone ones. Undo takes the newest done record back and puts it on the undone stack; Redo puts the newest undone record back and returns it to the done stack; a new command on the history clears the undone stack. Both are the session's memory alone, not a second log: they end as the photo closes, as darktable's end on leaving the darkroom.
- **The fallback.** Only when the done stack is empty, as after reopening a photo, does Undo move the end back through the steps the sidecar kept, over the step before it and the rest of its batch before that; only when the undone stack is empty does Redo move it forward over the next step and the rest of its batch. Each fallback move is recorded on the opposite stack, Undo's on the undone stack and Redo's on the done stack, so the next Redo or Undo reverses it, and is named as `Move::Steps` of the steps it crosses, not as a `GoTo`; a fallback move clears no undone record. With nothing to undo or redo they change nothing; the `History` fact says beforehand what they would do (`undo`, `redo`).
- **Going to a step** (`GoTo`) sets the end just after it, any step, one inside a batch among them, as darktable's click on any item, or to 0 for Original. The later steps stay, greyed, and are saved, so Redo's fallback still reaches them after reopening. Undo takes the click back, as darktable's does (`src/libs/history.c:1297-1318`, recorded by `:664-686`), and after a click and an edit, which drops the later steps, Undo brings them back while the photo is open. Going to where the end is already changes nothing and records nothing, as darktable's click on the item applied last does nothing (`src/libs/history.c:1265`).
- **An edit while the end is below the last step** drops the steps after the end, then adds its own, as darktable's `_dev_add_history_item_ext` removes the items above `history_end` (`src/develop/develop.c:1191-1229`). darktable keeps one kind above the end, moving it below: an always-on module's item (default-enabled, or with no on-off button) with no earlier entry, its first item of a new photo. Apertrust stores no such item, the modules every photo runs being the derived Default lines, so it has no counterpart. Until the photo closes, Undo brings the dropped steps back; after that they are gone.
- **Reset** is a step (above): Undo brings the edit back.
- **Compress** is darktable's compress history stack (`src/libs/history.c:1239-1242`, `dt_history_compress_on_image` in `src/common/history.c:1193`). darktable keeps, of the items below `history_end`, the last of each operation and instance and deletes every other item, those above the end among them (`src/common/history.c:1244-1255`), and at end 0 deletes the whole history (`:1212-1216`). So Apertrust's Compress keeps, among the steps up to the end, each module's and field's last step, with its id, clock, source, note and batch; drops one whose entry is empty, Reset's among them, since the reset state is what no entry gives; drops **every step after the end**, all of them at end 0; and makes no step. The end is then the last step kept.
- **Truncate** is darktable's Ctrl-click on the same button (`src/libs/history.c:1244-1250`, `dt_history_truncate_on_image` in `src/common/history.c:1348-1399`): it deletes the items at and above `history_end` and nothing else (`:1366-1376`), all of them at end 0 (`:1357-1362`). So Apertrust's Truncate drops the steps after the end, keeping every step up to it as it is, and makes no step.
- **Both are undoable:** darktable's `_lib_history_truncate` (`src/libs/history.c:1189-1235`) records an undo for each, and Undo puts the log back while the photo is open.
- **The clock.** Each step carries its own clock, and each move of the end (Undo, Redo, GoTo) and each Compress or Truncate, or its Undo or Redo, advances the copy's clock vector with a clock of its own, so two copies of the sidecar converge under [decision 52](decisions/0052-comparing-two-copies-of-a-sidecar.md).
- **A gesture under way** is ended as completed by Undo, Redo, GoTo, Compress and Truncate, as by any command on the edit (decision 42), so Undo during a drag takes back the drag.
- **A batch on several photos:** Undo on a batch takes it back on every photo where its last step is still the end, whichever of them is active; a photo changed since keeps it under its own steps, and the notice says so ([gui.md](gui.md#selection-and-the-active-photo), decision 37). After a restart the library's index finds the batch's photos by the batch id of each photo's last applied step. Planned with several photos in one session; until then a batch has one photo.
- **Leaving and reopening a photo:** the log and its end are in the sidecar, so Undo and Redo come back through their fallback, and so do the greyed steps; the undo records do not.
- **A crash** loses at most the gesture in flight: each completed change, Undo, Redo, GoTo, Compress and Truncate is handed to the edit writer at once (decision 47).
- **A newer file on disk.** Before it writes, the writer compares the file's size and modified time with those it last read or wrote. Where they differ, as after `apertrust-cli edit` wrote a step, it writes nothing; the session reads the newer file, lays the steps it has not yet saved on the newer log after its end, as an edit after Undo would, and sends `Values` and `History` again. The undo records, a held Compress and the merge state are cleared, since they describe a log that is gone. This is the part of the library's step 10 (read before every write, decision 52) that history needs; step 10 adds the rest.
- **A photo open for viewing only, or not yet live** ([decision 77](decisions/0077-a-photo-opens-in-stages.md)): its history is sent and shown; Undo, Redo, GoTo, Compress, Truncate and `Step` are commands on the edit, refused as every change is (`ViewOnly`, `NotLive`). `DiscardHistory` is the one exception, and only for an edit view-only because part of its history cannot be read (a `sidecar::Problem`), never for one a newer build wrote: warning first, so the person can keep a copy of the file, it writes a fresh log holding one step, source You, noted `history-discard`, whose entries are the edit as far as it was read, the end that step, and ends the view-only state; Original stays the untouched photo (decision 78, planned with 9.8).
- **A sidecar changed on another computer:** until the library's step 12, step 10 opens a diverged sidecar view-only; step 12 keeps the other side as a virtual copy with its own log (decision 52). Joining two logs step by step comes with step 14.

## Through the session

### Commands

All but `Hide` are commands on the edit (`on_edit`): each ends a gesture under way, a gesture being one module's changes of one source and note, but for a change that goes on with it or takes it in, of its source and note, and a `Step` refused as `DefaultSource` or `EmptyStep`, which leaves it under way; each is followed by a `Values`, and is refused on a photo open for viewing only or not yet live.

| Command | What it does |
| --- | --- |
| `Undo` | Takes back the newest done record, a batch's steps whole; only when there is none, moves the end back over the step before it and its batch |
| `Redo` | Puts back the newest undone record; only when there is none, moves the end forward over the next step and its batch |
| `GoTo { step: Option<StepId> }` | The end just after `step`, one inside a batch among them, or Original with `None`, once a change not yet a step is made one; a step not in the log then refused, `UnknownStep` |
| `Compress` | Compress History, as above, darktable's compress |
| `Truncate` | The steps after the end dropped, as above, darktable's Ctrl-click on compress |
| `Reset` | The whole edit back to nothing, as one step noted `history-reset`, holding every module it reset |
| `DiscardHistory` | Planned with 9.8: Discard History…, darktable's discard, the log cleared and the edit reset, as going to Original and then Compress, one undo record; on an edit view-only because of a `sidecar::Problem`, the one command allowed, a fresh log of a step of each module the edit holds as far as it was read, sharing a batch, noted `history-discard` |
| `Step { source, note, commands }` | Several commands on the edit as one change with that source and note, a step of each module they write, in a batch where there are several |
| `Hide { module, hidden }` | The eye: the live frame shown as if the module were reset; no step, nothing written |

**`Step`** is how a client names a source and a note:

- it holds only `SetControl`, `ResetControl`, `ResetModule`, `Keep`, `ApplyPreset`, `Set` and `Reset`;
- they are applied in order to the edit in place, all or none: the edit, its held coordinates and whether the writer has it are kept aside and put back should one be refused, so one refused refuses the whole and changes nothing: `SessionError::Refused { command, error: InStep { index, error } }`, naming the `Step`'s number, the failing command's index, counting from 0, and its error;
- any other command inside, a `Step` among them, is refused the same way at its index, `NotInStep`;
- `source` cannot be `StepSource::Default`, which the session alone writes: refused, `DefaultSource`; a `Step` holding no command is refused, `EmptyStep`; both before the gesture under way is touched, so it goes on;
- it completes as one change when it holds any command but `SetControl { done: false }`, a step of each module it writes, sharing a batch id, which one Undo takes back; one of `done: false` changes alone is a gesture's change of its source and note, on the module of the last of them;
- **one source per gesture:** a gesture has the source and note of the change that began it. A `done: false` change on its module of another source or note ends it as a step of its own and begins another with its own; a completed change takes it in, a release on its module or a `Step` writing it, only with its source and note, and otherwise the gesture is a step of its own first. A `Step` refused begins no gesture, and one that would have taken the gesture in leaves it a step of its own, as a release refused does;
- a client's plain commands are steps of source You with no note;
- the app sends Auto's writes as `Step { source: Auto }` (a picker's are the session's own, [pickers.md](pickers.md#what-a-pick-writes)), a style's as `Step { source: Style, note }` (Phase 3), and its own changes of several commands as `Step { source: You }`; the MCP adapter sends one `Step { source: Assistant, note }` per request (Phase 6).

**A batch id is the session's,** never a client's: one change of several modules gets one `BatchId`, written into each of its steps, and one command acting on several photos will get one, written into each photo's steps. A client cannot make two requests one batch, nor two clients collide on an id.

**`Hide { module, hidden }`** (the eye) is view state, as `Resize` and `Scopes` are. While hidden, the live frame renders as if the module were reset: off for a switched module, its defaults for one every photo runs, so the eye on an untouched module every photo runs changes nothing. Its operation is never skipped: for White balance or the tone mapper that would show an image no edit can make. It is no step, writes nothing, changes no export, still or thumbnail, and ends as the photo closes or another opens. `Values` reports it as `ModuleState::hidden`. A layer's eye comes with layers.

### The `History` event

`Event::History(History)`, a fact (`Fact::History`), the whole topic each time ([decision 59](decisions/0059-one-event-system.md)):

```rust
pub struct History {
    /// How many commands it includes, as `Values::through` counts them.
    pub through: u64,
    /// Oldest first; a client lists them newest first. Each behind an `Arc`, so
    /// facts sent one after another share the steps they have in common.
    pub steps: Vec<Arc<HistoryStep>>,
    /// How many steps apply, darktable's `history_end`: 0 is Original, and the
    /// step numbered `end`, counting from 1, is the last applied.
    pub end: usize,
    /// The Default lines' modules, in darktable's history order for a new photo: no
    /// on/off button, then one, then the rest, each group in the pipe's order; their
    /// values are each control's default in `Values`.
    pub defaults: Vec<&'static str>,
    /// What Undo and Redo would do now, so a menu names it; `None` when nothing.
    pub undo: Option<Move>,
    pub redo: Option<Move>,
}

pub struct HistoryStep {
    pub id: StepId,
    pub clock: Clock,
    pub source: StepSource,
    pub note: Option<Note>,
    /// Shared by the steps of one change of several modules, which Undo and
    /// Redo move whole.
    pub batch: Option<BatchId>,
    /// The module the step touched, with what changed: one, or for a reset
    /// each module it reset.
    pub changes: Vec<ModuleChange>,
}

pub struct ModuleChange {
    pub module: &'static str,
    /// Before and after, where the step turned the module on or off.
    pub enabled: Option<[bool; 2]>,
    pub controls: Vec<ControlChange>,
}

pub struct ControlChange {
    pub control: &'static str,
    pub before: ControlValue,
    pub after: ControlValue,
    pub mark: [Option<Mark>; 2],
}

pub enum Move {
    /// One change's steps, a batch's whole, added, taken back or put back, or
    /// the end moved over them; a batch's part where `GoTo` left the end
    /// inside it.
    Steps { first: StepId, count: usize },
    /// A `GoTo` taken back or put back.
    GoTo,
    Compress,
    Truncate,
}
```

- **The types** live in `apertrust_engine::history`, re-exported by the session, plain data that moves into `apertrust-protocol` unchanged when it is made. `Clock` and `MachineId` are `apertrust_library::clock`'s, built with step 8's first part. `StepSource`, `StepId`, `BatchId` and `Note` are written by name or as text on the wire, a client ignoring a source it does not know.
- **When it is sent:** at a photo's open, right after its `Values { live: false }` (decision 77's stage 2); in each batch whose commands changed the log, its end or the undo records, once, after the batch's `Values` and before its `Frame`; and when the session takes a newer file from disk. A gesture's `done: false` changes change no step and send none.
- **Whole, not a delta.** A fact is always its whole topic (decision 59), a remote client is sent the latest and a client joining late needs nothing more. It is sent once per completed change, never per frame of a drag, and in process the steps it shares with the last are not copied. A step with one control changed is about 200 bytes in memory and 160 as JSON, so 200 steps make about 40 kB and 1,000 about 200 kB, estimated, a curve's points more. Deltas would need ordered notices, `Sync` replaying them and a merge in every client, for a topic smaller than one frame.
- **Its budgets:** at 200 and at 1,000 steps on the M4, building the fact takes under 1 ms, and its JSON, as the remote protocol will encode it, is under 256 kB. A measurement that misses its budget brings the planned fallback: the fact holds the step range the client shows, which it names with a command, the count and the end always whole, so a client still knows where the end is and what Undo would do.
- **Measured in 9.1** on the M4, release build, each step one control changed, three runs alike (`the_fact_s_budgets` in `session/history/tests.rs`, ignored, run by hand):

  | Steps | A batch's fact | Every step described anew | JSON, serde's derive | Its `null`s left out |
  | --- | --- | --- | --- | --- |
  | 200 | 0.007 ms | 0.08 ms | 59 kB | 51 kB |
  | 1,000 | 0.014 ms | 0.35 to 0.40 ms | 295 kB | 255 kB |

  A batch describes only the steps it added, the rest shared behind their `Arc`s, so the time is well within its budget. The JSON is an estimate, the test's imitation of serde's derive, nothing in process serialising the fact: a step is about 295 bytes, not the 160 estimated, its id, clock and source as text, its note, batch and `enabled` as `null`, and both values and marks named.
- **So the fallback is planned with the remote protocol,** the fact's only reader that serialises it: 295 kB at 1,000 steps, and 255 kB with its `null`s left out, miss the budget, so there the fact holds the step range a client names, with the count, the end, `undo` and `redo` always whole. Leaving out a field that is `None` (serde's `skip_serializing_if`) is a wire convention of `apertrust-protocol`; field names are not shortened.
- **The app's mirror** keeps the latest, by its `through`, and drops a replaced open's as it drops its values (`PhotoState::stale`).

**Gaps left for later, on purpose:**

- `History` names no photo or copy: it is the active photo's, its open's number ordering it. With several photos (decision 37) and virtual copies (step 11) it gains the photo's `PhotoRef`, its copy included.
- `Step` names no photos: with several photos it gains them, a batch's.
- `ModuleChange` is keyed by module alone: layers (Phase 4) add a layer and an instance to its key.
- An unowned operation's parameters, while `Set` exists, show as a `ModuleChange` under the operation's id, its controls the parameters, an integer as a number.
- **Built:** `Move::Steps` and `Move::GoTo`, `Compress` and `Truncate` coming with their commands. A move of the end by Undo's or Redo's fallback names the steps it takes back or puts back, as `Move::Steps`.
- A module this build does not know, in a photo open for viewing only, has no `&'static str`: such entries are listed by their stored name once `module` is widened to hold one, part of 9.6's scope, where the History tab first shows a view-only photo's steps.

## In the sidecar

History is part of `.apxmp` version 2 (decision 47; [library.md](library.md#version-2)), written through the XMP layer ([decision 50](decisions/0050-our-own-xmp-layer.md)) in one fixed layout, read and written back byte for byte. **Its target is conformance to ISO 16684-1:2019 (which revises ISO 16684-1:2012, whose text Adobe publishes free as the XMP Specification Part 1),** XMP Part 1: data model, serialization and core properties, as the XMP layer's is, held by the layer's fixtures and by 8.2's fixture below: every property is in a namespace declared with its URI, `apertrust`, `control` and `param` among them (decision 48), arrays are `rdf:Seq` and `rdf:Bag`, structures are nested `rdf:Description`s, and times are XMP Date values, so ExifTool, exiv2 and other XMP readers read the whole file.

Built with step 8.2, the text layer's `sidecar::Edits` (`crates/library/src/sidecar/copies.rs`). **The file** holds `apertrust:version` 2, `apertrust:photo`, the photo's file name, `apertrust:fingerprint`, its size, a colon and 64 hex digits ([library.md](library.md#fingerprints)), and `apertrust:copies`, an `rdf:Seq` of its virtual copies. It declares a fourth namespace, `https://apertrust.org/ns/xmp/arg/` (`arg`), for notes' arguments ([decision 48](decisions/0048-the-apxmp-format.md)).

**Each virtual copy** holds `apertrust:id`, 8 hex digits, `apertrust:number`, `apertrust:name` (empty for none), `apertrust:end`, the id of the last step applied (empty for Original), its clock vector, `apertrust:clocks`, an `rdf:Bag` of one clock per machine, written in the order of their machine ids so two machines writing the same copy write the same bytes (decision 52), and `apertrust:history`, an `rdf:Seq` of its steps, oldest first. An empty array is closed at once (`<rdf:Bag/>`), as ExifTool reads the whitespace inside an open one as text. Each step holds, in this order:

- `apertrust:id`, 16 hex digits;
- its clock in three fields: `apertrust:when`, the time as an XMP Date in ISO 8601 with milliseconds in UTC (`2026-03-14T14:02:11.120Z`), so other tools read it as a date; `apertrust:counter`, the hybrid logical clock's counter; and `apertrust:machine`, the machine id. A clock in the copy's vector is the same three fields;
- `apertrust:source` by name: `you`, `auto`, `style`, `assistant` or `default`;
- its note: `apertrust:note`, the words as written, or `apertrust:noteMessage`, the message id;
- `apertrust:batch`, 16 hex digits, when it has one;
- a message's arguments, where it has any: `apertrust:noteArgs`, a struct whose fields are the arguments in the `arg` namespace by their names (`<apertrust:noteArgs><rdf:Description arg:style="Film grain"/></apertrust:noteArgs>`). Nested, an argument named as a step's own field, `id` or `when`, stays apart from it in tools that name a struct's fields by their local names alone, as ExifTool does;
- `apertrust:entries`, one `rdf:Seq` of its modules' and unowned operations' entries in the step's order, each written as version 1 writes it.

Every value is read as the text the file holds but the clocks. What cannot be read as written is read as far as it can be and told to the caller as a `sidecar::Problem` in `Edits::problems`, as decision 48 leaves out or clamps an unreadable value with a warning and the photo opens: a step's clock that cannot be read keeps the step with no clock; a clock of the vector that cannot be read is left out; a step with both `apertrust:note` and `apertrust:noteMessage` reads the message; either that is no plain text (a struct, an array, a resource or a text with qualifiers) is left out; `apertrust:noteArgs` without `apertrust:noteMessage` is left out, and so is whatever `apertrust:noteArgs` holds that is not a text argument (a field of another namespace, an argument that is no plain text, `noteArgs` that is no struct). An edit with any problem opens for viewing only, so nothing is written over what this build could not read. A file of a version above 2 is read as version 2 lays it out, and `Edits::newer` names it. A whole file with one step:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Apertrust 0.5.0+3~g1a2b3c4">
 <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
  <rdf:Description rdf:about=""
    xmlns:apertrust="https://apertrust.org/ns/xmp/"
    xmlns:control="https://apertrust.org/ns/xmp/control/"
    xmlns:param="https://apertrust.org/ns/xmp/param/"
    xmlns:arg="https://apertrust.org/ns/xmp/arg/"
    apertrust:version="2"
    apertrust:photo="DSCF0001.RAF"
    apertrust:fingerprint="30482117:4d0ef7895cd32cf78b2af8b9f7a8ee88feb39286faf31897966133fdde7b601d">
   <apertrust:copies>
    <rdf:Seq>
     <rdf:li>
      <rdf:Description
        apertrust:id="7c0e9a14"
        apertrust:number="0"
        apertrust:name=""
        apertrust:end="a1b2c3d4e5f60718">
       <apertrust:clocks>
        <rdf:Bag>
         <rdf:li>
          <rdf:Description
            apertrust:when="2026-03-14T14:02:11.120Z"
            apertrust:counter="0"
            apertrust:machine="5e1f22c9"/>
         </rdf:li>
        </rdf:Bag>
       </apertrust:clocks>
       <apertrust:history>
        <rdf:Seq>
         <rdf:li>
          <rdf:Description
            apertrust:id="a1b2c3d4e5f60718"
            apertrust:when="2026-03-14T14:02:11.120Z"
            apertrust:counter="0"
            apertrust:machine="5e1f22c9"
            apertrust:source="you">
           <apertrust:entries>
            <rdf:Seq>
             <rdf:li>
              <rdf:Description
                apertrust:module="exposure"
                apertrust:moduleVersion="2"
                apertrust:kept="exposure"
                control:exposure="1.2"/>
             </rdf:li>
             <rdf:li>
              <rdf:Description
                apertrust:operation="highlights"
                apertrust:schema="1"
                param:clip="0.9"/>
             </rdf:li>
            </rdf:Seq>
           </apertrust:entries>
          </rdf:Description>
         </rdf:li>
        </rdf:Seq>
       </apertrust:history>
      </rdf:Description>
     </rdf:li>
    </rdf:Seq>
   </apertrust:copies>
  </rdf:Description>
 </rdf:RDF>
</x:xmpmeta>
```

- **A fixture holds it:** `crates/library/tests/fixtures/xmp/version-2-history.apxmp`, one virtual copy with five steps, the end on the fourth, every source but Default, a note in words, a message with its arguments, one of them named `id`, a batch, a switched module on, an unowned operation's entry and two machines' clocks. The text layer writes it back byte for byte. ExifTool 13.55 (`exiftool -a -G1 -struct`) and exiv2 0.28.9 (`exiv2 -px`) read it whole, without error or warning, ExifTool reading each `apertrust:when` as a date and showing it in its own form (`2026:03:14 14:02:11.120Z`) and the argument `id` apart from the step's (`NoteArgs={Id=…}`). A copy at Original, with no clock and no step, written by the text layer, reads in ExifTool (`exiftool -j`) with its empty arrays empty. The tests run each tool where it is installed and skip it where not (`tests/sidecar_fixtures.rs`).
- **The end is stored as a step's id,** not a count, so it stays right when two logs join by clock (step 14); the protocol's `end` is its count, worked out as the log is read.
- **Version 1 reads as a step of each module,** modules first, then each operation's unowned parameters, each in id order, whatever the file's order: source You, no note, sharing a batch where there are several, so one Undo takes the edit read back whole; the first step's id the first 16 hex digits of the BLAKE3 hash of the file's bytes, each next one more, and the batch's the next 16, so every machine reading the same file gives the same steps; their clock's time the file's modified time, its counter 0 and its machine id `00000000`, unknown; the end after the last; a file with no module and no parameter reads as no step, end 0. Version 0 reads through version 1's reading, as now. Such a file is written as version 2 only at the next change. Two readers do this today, giving the same steps, ids and batch for every file Apertrust writes: the engine's `Log::read` reads it from the typed edit, dropping each entry at its reset state, and the text layer's `Edits::parse` keeps every entry the file holds, as Apertrust writes none at its reset state; 9.5 keeps `Edits::parse` as the one conversion and retires `Log::read`. `Edits::parse` reads it as one virtual copy, `sidecar::FIRST_COPY`, id `00000000`, number 0 and no name, the same on every machine so that two machines changing the same version 1 file compare one copy (decision 52), its history those steps, its end the last step's id and its clock vector their clock; a file holding no entry reads as the copy with no step, its end empty.
- **Every entry of every step** follows decision 48's rules, not only the edit at the end, since a save writes them all: a newer module version or an unknown module anywhere in the log makes the photo view-only, and an older one is migrated entry by entry.
- **Compress and Truncate in the file** leave only the steps they kept, and the copy's clock vector advanced with their own clock.
- **No limit on the number of steps,** as darktable has none; Compress and Truncate are how the person shrinks a log. A step is about 100 to 300 bytes (decision 47), so 1,000 steps make about 100 to 300 kB. With the sidecar's build, a 1,000-step log's bytes and its read and write times are measured on the M4's SSD and over SMB to the NAS, against reading it within 20 ms and writing it within 50 ms on the SSD; a miss blocks 9.5 until it is met.
- **The index** counts a photo edited when some module's last entry up to the end is not empty, a fold of the entries as text, and keeps the history's length for Filters' History (decision 73).

### Step 8 first, in two parts

History needs two parts of the library's step 8, and they come first:

- **8.1, the clock and the machine id,** built in `apertrust-library`'s `clock`, before the log, so every step carries its real clock from its first build. The system's machine id is read through the safe crate `machine-uid`, with no `unsafe` of Apertrust's ([library.md](library.md#version-2)).
- **8.2, version 2's text layer,** built in `apertrust-library`'s `sidecar` before history is saved: the photo's identity (`apertrust:photo` and `apertrust:fingerprint`), and `apertrust:copies` with its virtual copies, each with its id, number, name, clock vector, log and end, each time an XMP Date; version 1 read as a step of each module, sharing a batch; the history fixture ExifTool and exiv2 read; and the clocks' drift bound, `Clock::observe` ignoring another machine's clock read more than 5 minutes past the wall clock ([decision 47](decisions/0047-where-edits-live-and-what-the-sidecar-holds.md)).

**Why history does not land in a version 2 of its own first:** history belongs inside `apertrust:copies`, one log per virtual copy. Holding it elsewhere first and moving it into `apertrust:copies` with step 8 would change how existing version 2 files read, against decision 48's one fixed layout per version: a third version, and builds between the two layouts, nightly ones among them, opening each other's edits view-only.

**What of step 8 remains for later:** the engine writing version 2 (9.5); machines' names in a library's `.apertrust/machines/` (Phase 5); the fingerprint's uses, re-pairing a renamed raw (step 18) and keying thumbnails (step 6); the commands for more than one virtual copy, and tombstones (step 11); comparing two copies by their clock vectors (steps 10 and 12).

## The CLI

`apertrust-cli` works on the sidecar directly through the engine's `history` and `sidecar`, as `edit` does today, never through a session. The app's session finds such a write before its own and takes the newer file ([Behaviour](#behaviour)).

- **`edit FILE`** adds one change for all its flags, source You, with `--note TEXT` for its note: a step of each module its flags write, sharing a batch, as the session's change of several modules is. `--clear` resets the edit within that change, the log kept. An edit while the end is below the last step drops the later steps, as in the app.
- **`history FILE`** (planned) lists the steps newest first, the end marked, each with its number, source, note, time and changes, before → after; `--undo [N]` and `--redo [N]` move the end, `--to STEP|original` goes to a step, `--compress` compresses, `--truncate` drops the steps after the end and `--discard` discards the history as `DiscardHistory` does, an edit view-only because of a `sidecar::Problem` included. Each move advances the copy's clock vector, as the session's do. The CLI keeps no undo records, so its Undo and Redo move the end. An edit a newer Apertrust wrote is refused, as `edit` refuses it.

## In the app

- **Edit › Undo and Redo** and their keys (Ctrl+Z, Shift+Ctrl+Z, Ctrl+Y off the Mac; [gui.md](gui.md#the-menus)) send `Undo` and `Redo`, enabled from the mirror's `History::undo` and `redo`, naming what they would do as History names it ("Undo Exposure · Sky", "Undo Go to Step", "Undo Compress History", "Undo Truncate History"; `menu-undo-step`). In a focused text field they act on the field (decision 39).
  - Built (9.4), on the Mac's bar and in the ☰: enabled while the photo is live and not open for viewing only, on the main window with no modal dialog over it, and repeating while a key is held; the top panel's Undo and Redo run them too, enabled from the history alone, and keep the plain words Undo and Redo until the History tab (9.6).
  - The words of the steps they would move (`history::steps`, the app's), named only while the photo takes edits: their note, which a batch's steps share (`history-reset` reads Reset), else the title of the first step's module the app knows, with how many more it knows for a batch ("Undo Exposure and 2 more", `history-modules`); a `GoTo` is `history-go-to-step`. Steps none of whose modules the app knows, as an operation's unowned parameters alone, read Undo alone. darktable's label after a • and a layer after a · join with layers.
  - A focused text field keeps the keys for its own undo, and the Mac's items keep them too: performed or chosen from a menu while it has the focus, Undo and Redo read Undo and Redo and hand the field egui's own keys for the next frame.
- **The History tab** draws `History` ([The History tab](#the-history-tab)): a line per step from `changes` and `note`, the tag from `source`, the tooltip from `changes`, `note` and `clock` (dates and times through `icu_datetime`), the end's later steps greyed; a click on a step sends `GoTo`, Shift-click and a click on a Default line open the module, the eye sends `Hide`, Compress history sends `Compress`, Ctrl-click on it (Cmd-click on the Mac) sends `Truncate`, and Reset sends `Reset`. Create a style… comes with styles (Phase 3).
- **Changes of several commands** go as one `Step { source: You }`: the tone equalizer's release and its reset of the nine bands.
- **Reset Edit and Discard History…** have no default key, as darktable's discard history has none (its button registered with no key, `src/libs/copy_history.c:328-330`; the history module's reset asking first, `src/libs/history.c:1335-1354`), since it throws the whole edit away at once; a person may bind one in Settings › Shortcuts, as every action. The registry's comment on `reset` says so.

## Against darktable

| darktable 5.6.2 | Apertrust | Why |
| --- | --- | --- |
| A history list saved with the image, and an undo stack in memory of the whole history and its end before each change, lost on leaving the darkroom (`src/common/undo.c`, `src/libs/history.c:664-686`) | One log saved in the sidecar, and done and undone records in the session's memory, lost as the photo closes; only with no record, Undo and Redo move the log's end, each such move recorded for the other to reverse | Undo survives reopening and a crash (decision 47) |
| Undo of a change restores the history without its item; Redo brings it back | The same: Undo takes the step out of the log, any steps it dropped coming back | Kept |
| History items merge a module's consecutive changes until another module takes focus (`_dev_add_history_item_ext`) | Steps merge only as undo merges a widget's changes | History and Undo agree |
| A history item is one module's (`develop.h:34`); a style, a copy or a paste adds an item per module in one undo group (`styles.c:954`, `history.c:157`) | The same: a step of each module, sharing a batch id, which Undo and Redo move whole; `GoTo` lands on any step | Kept |
| Undo merges changes to the widget changed last, a move or a reset alike, within 10 s of the record's start unless 2 s have passed since the pipe finished the last (`develop.c:1351-1370`, `:885`); only widgets wired through `dt_iop_gui_changed` merge, so changes through a module's own callbacks never do, White balance's settings, finetune, temperature and tint among them (`develop.c:1359`; `src/iop/temperature.c:1712`, `:1782-1945`, `:2198-2201`) | The same, by target, every control and preset row alike, White balance's among them; the 2 s from the first frame published that includes the step's last change | One rule for every module, whatever its controls are wired through (decision 78) |
| An edit below `history_end` deletes the items above it, but moves an always-on module's item with no earlier entry below the end (`develop.c:1191-1229`) | The steps above the end dropped; no stored item of an always-on module exists to keep | The modules every photo runs are the derived Default lines |
| A click on a history item is undone by Ctrl+Z (`history.c:1297-1318`) | The same, while the photo is open | Kept |
| Compress keeps the last item per operation and instance below `history_end` and deletes the rest, those above the end among them, the whole history at end 0, undone by Ctrl+Z (`src/common/history.c:1212-1216`, `:1244-1255`) | The same, its empty entries dropped, since an empty entry and none are the same reset state | Kept |
| Ctrl-click on compress truncates: the items at and above `history_end` deleted, the rest kept, undone by Ctrl+Z (`src/common/history.c:1366-1376`) | `Truncate`, Ctrl-click or Cmd-click on Compress history | Kept |
| A new photo's history lists the modules on by default with no on/off button, then those with one, each in the pipe's order, then the workflow's auto-applied presets in the order its preset query returns them (`develop.c:2142-2171`, `:2343`) | The Default lines in the same three groups, the third in the pipe's order | The third group is no presets in Apertrust, but the modules every photo runs |
| Original shows the image without its auto-applied modules | Original is the untouched photo, darktable's defaults included; the Default lines open their modules | The modules every photo runs cannot be turned off (decision 32) |
| Discard history removes the history, asking first unless told not to (`ask_before_discard`), undone by Ctrl+Z until leaving the darkroom; no default key | Discard History… (`DiscardHistory`) the same, as going to Original and then Compress, undone through its one undo record while the photo is open; no default key. Reset, a step that resets the edit with the log kept, beside it | Kept; Reset is undoable after reopening too (decision 47) |
| A reset leaves no item of its own: discard deletes the items, and a module's reset is that module's item | Reset is one step holding an empty entry for every module it reset, one Reset line | One line says what was done, where a step of each module would list every module the edit held |
| `history_end` stored as a count | The last applied step's id | Logs that join by clock (step 14) |

## Where the code lives

| Part | Where |
| --- | --- |
| The log, steps, fold and merge, built; compress, planned | `apertrust-engine`'s `history` |
| Commands, `Event::History`, the undo records, the gesture and merge clocks, the newer file taken in | the engine's `session` |
| Version 2's copies and steps as strings | `apertrust-library`'s `sidecar`, built with step 8.2 (`sidecar::Edits`) |
| Reading and writing steps with the module definitions | the engine's `sidecar` |
| The clock and the machine id | `apertrust-library`'s `clock`, built with step 8.1, its drift bound with 8.2; the system's machine id read through `machine-uid` |
| Undo and Redo, the History tab, the eye, changes of several commands as one `Step` | the app's `actions`, `menus`, mirror, modules and a History section |
| `edit` and `history` | `apertrust-cli` |

## Tests

**Built with 9.1:**

- **The log** (`history/tests.rs`): a change's entries holding each changed module's whole state, an empty entry its reset; the fold, each module's last entry up to the end; a step added below the end dropping the steps after it; the steps described control by control, before → after, a switched module's on and off and an unowned parameter, each described once.
- **The session's history** (`session/history/tests.rs`): over 40 seeded random sequences of 300 changes, completed changes, Undo, Redo and reopening, which clears the records so Undo and Redo fall back, half of them from an edit read from a file, the fold always the edit, every step's fold the edit as it left it, and Undo and Redo each other's inverse; Undo taking a step out of the log and Redo putting it back; an edit below the end dropping the later steps and Undo bringing them back; the fallback mixed with records; a change not yet a step made one before Undo; the budgets measured (`the_fact_s_budgets`, ignored).
- **The session** (`session/tests/history.rs`): the `History` after an open's values not yet live; one `History` per batch that changed it, after `Values` and before `Frame`, none during a gesture, `through` never decreasing, shared steps the same `Arc`s; Undo and Redo rendering and saving each other's inverse; Reset a step, turning a switched module off, its Undo bringing each back; a pending `Set` and a drag taken back by Undo, a gesture another command ends, or whose release is refused, a step of its own; after reopening, the fallback and an edit dropping the step read, which Undo brings back; `Undo` and `Redo` refused while not live and view-only.

**Built with 9.2:**

- **The session's history** (`session/history/tests.rs`): the random sequences with `GoTo`s among them, each landing on the edit its step left, or Original's, and Undo and Redo taking it back and putting it back; `GoTo` and Undo, after it and after it and an edit, which brings the dropped steps back, then the end; Original, the end where it is, and an unknown step refused.
- **The session** (`session/tests/history.rs`): a `Step` of its source and note, which Undo takes back whole; a `Step` refused whole with its index, writing nothing, a command not on the edit's values inside refused as `NotInStep`; `Default` refused, beginning no gesture, and no step of `Default` following; an empty `Step` refused, the gesture under way going on; a refused `Step` beginning no gesture, one going on with the gesture leaving it under way, one of another source ending it as a step of its own, and one that would have taken the gesture in leaving it a step of its own, the edit as the gesture left it; a `Step` taking in the gesture on a module it writes, and one on another module leaving the gesture a step of its own; a `Step` of `done: false` changes alone a gesture of its source; one source per gesture, a change of another source or note on its module ending it or not taking it in; `GoTo` rendering and saving, Undo and Redo taking it back and putting it back, Undo after it and an edit bringing the dropped steps back, Original, and the end where it is changing nothing; `GoTo` with an unknown step refused; `Undo`, `Redo`, `GoTo` and `Step` refused while not live and view-only.
- **The app:** the tone equalizer's release of several bands and its reset of the nine, by a double-click and by Shift+Delete, each one `Change::Step` (`modules.rs`), which `main.rs`'s `sent` turns into one `Step { source: You }`, each change shown as it would be alone, the save state waiting for it as a completed change, but not for a `Step` of a gesture's changes alone, as the engine says (`changes_of_several_commands_are_sent_as_one_step`).

**Built with 9.3:**

- **The log** (`history/tests.rs`): a merged change taking the place of the last applied step's entry, keeping its id, its clock the merge's.
- **The session's history** (`session/history/tests.rs`): the random sequences with each change writing a target at a random time and frames published, some merging, each merge adding no record, the fold always the edit and Undo and Redo each other's inverse; a merge by the two times and a split at each boundary, the 10 s counted from the step's first write however many changes merged since and the 2 s from the first frame including the last; one Undo taking back a merged step whole; no merge across another control, a preset row, another source, a note, several targets or none, a command between, or Undo and Redo, and a step of Style, Assistant or with a note merged into by none; a cancelled gesture forgotten.
- **The session** (`session/tests/history.rs`), on injected command and frame times: a control's quick repeats one step keeping its id, described from before the first to after the last, split at 10 s from its first and at 2 s from the frame of the last, a reset of the control merging too, and one Undo taking back each merged step whole; no merge across another control, a refused command, a `Step` refused as `DefaultSource` or `EmptyStep`, a keep, a note, another source or a `Step` of several commands, Auto's repeats merging and a preset row's too; a change that changes nothing, a gesture's on another control or a `Set`'s, counting as none, the next change still merging.

**Built with 9.4:**

- **The mirror** (`mirror.rs`): `the_mirror_keeps_the_photo_s_latest_history`, the latest by its `through`, one with the same `through` replacing it, none before a photo, while another opens or once it closes or fails, an earlier open's dropped.
- **The words** (`history.rs`): a step named by its module's title, its note, `Go to Step`, and nothing it cannot name, in en-GB, uk and pl.
- **The actions** (`actions.rs`): Undo and Redo enabled from each side of the history or a focused text field, resting under a modal dialog and in a dialog's own window; the keys a text field is handed.
- **The menus** (`menus.rs`): the items' words naming the step, Undo alone with nothing to name and while a text field has the focus, the Mac's words the same, and enabled from the history; the Mac's items keeping Cmd+Z and Shift+Cmd+Z while a text field has the focus, and holding none while a dialog's own window has it.
- **The app** (`main.rs`): Cmd+Z and Shift+Cmd+Z on the Mac, Ctrl+Z, Shift+Ctrl+Z and Ctrl+Y on Windows, and the top panel's Undo, sending `Undo` and `Redo` once there is a step, a held key repeating, nothing without one; the ☰'s Edit naming the steps and sending the commands, Redo showing Ctrl+Y on Windows and Ctrl+Shift+Z on Linux; a text field's own undo by its keys and from the menus, Edit › Undo from the ☰ opened by F10 among them, the photo's history sent nothing.

**Built with 8.2:**

- **The clock** (`clock/tests.rs`): a clock read within 5 minutes past the wall clock honoured, the next clock after it, at the bound too; another machine's past the bound, an hour ahead or dated 9999, ignored, the next clock the wall clock's again; this machine's own clock honoured after its wall clock is set back an hour, the next clock after its own step; observing never moving the clock back.
- **The text layer** (`sidecar/tests.rs`): version 2 read back as written, a note in words, a message with its arguments in their struct after the batch, a batch and a step's entries in its order among it; the clock vector written in the order of its machines; an empty file and a copy at Original; version 1 read as a step of each module, modules first and each in id order, in a batch, the same ids on every read and others for other bytes, written as version 2; version 1 and 0 holding nothing read as a copy with no step; a version above 2 read as version 2 lays it out and named newer; a step's clock that cannot be read keeping the step with no clock, a vector's left out, each a problem; a note beside a message, arguments without a message, and note arguments of another namespace, not text or not in a struct, and a note or message that is no plain text, problems; a step written with no clock reading back the same; a file read with its modified time; the fingerprint, the size and the hash of a file's first and last 64 KiB, its middle unread.
- **The XMP layer** (`xmp/tests.rs`): an empty array closed at once.
- **The fixture** (`tests/sidecar_fixtures.rs`): written back byte for byte, read by ExifTool and exiv2 where installed, an argument named `id` kept apart from the step's; a copy at Original read by ExifTool with no text in its empty arrays.

**Built with one module a step:**

- **The log** (`history/tests.rs`): a change a step of each module and operation in the order written, those no command named after them, and a `Reset` one step in its place holding what no later command wrote (`a_change_is_a_step_of_each_module_in_the_order_written_and_reset_one`); the fallback's bounds over a batch, from its end and from inside it, and the fold inside it (`the_fallback_moves_over_a_whole_batch`); a merge into a step of one module only, none into another module's or a batch's; version 1 read as a step of each module sharing a batch, the same ids on every read; a batch's steps and a Reset's described.
- **The session's history** (`session/history/tests.rs`): the random sequences with changes not yet a step going on over several rounds, so some touch several modules, each change's fold, up to its last step, the edit it left, a step inside a batch changing only the batch's modules up to it, Undo taking back batches whole and `GoTo` landing inside them; a change of two modules two steps in one batch, in the order written, of its source and note, which one Undo takes back and one Redo puts back, and the fallback moving over both; a batch's step merged into by none; `GoTo` inside a batch, Undo and Redo of it, the fallback over the part applied and the rest, and an edit there dropping the rest, which Undo brings back.
- **The session** (`session/tests/history.rs`): a `Step` writing two modules two steps in one batch, `undo` and `redo` naming both, one Undo taking both back and, after reopening, the fallback both; Reset one step in no batch.
- **The app** (`history.rs`): a batch named by its note, else its first step's module and how many more, the part `GoTo` left of one alike, in en-GB, uk and pl.

**Planned, with each part:**

- **The log:** Compress keeps each module's last entry, drops empty ones and every step after the end, and renders the same; Truncate drops the steps after the end alone, at end 0 all of them.
- **The session:** Undo after Compress and after Truncate restoring the steps and the end; `Compress` and `Truncate` refused while not live and view-only; `Hide` changing no file and no export; each move advancing the clock vector.
- **The sidecar:** a newer module version in an old step making the photo view-only; reopening restoring the steps and the end; a writer stopped after a completed change losing nothing but the gesture in flight; a file the CLI wrote meanwhile taken in, the session's unsaved step laid on it; the 1,000-step measurements.
- **The app:** the History tab's snapshots, light and dark.
- **The CLI:** `edit` adding a step of each module it writes, in one batch; `history`'s moves, `--compress` and `--truncate` on fixtures.

## The plan

The library's step 9 ([roadmap.md](roadmap.md#the-librarys-steps)), one pull request each, each reviewed before the next. **Switched modules (decision 32) come first, as a prerequisite:** history relies on them throughout, for a reset step's `off`, an entry's `enabled`, Reset turning modules off and the eye's reset state, with step 8's two parts where history needs them. 9.1 to 9.4 change nothing in the file.

| # | Delivers | Touches | Out of scope |
| --- | --- | --- | --- |
| 8.1 | Built: the hybrid logical clock and the machine id in `apertrust-library`'s `clock`, the system's machine id read through `machine-uid` with no `unsafe` of Apertrust's | library | version 2 |
| 9.1 | Built: the log in the session's memory: `history`'s `Log` and `Step`, each completed change a step of each module it touched, several in a batch, `Reset` one step with its note, a pending `Set` made a step, `Undo` and `Redo` with the undo records, `Event::History`, its build time and JSON size measured at 200 and 1,000 steps on the M4 against their budgets; the file still version 1, the edit at the end | engine | `GoTo`, `Step`, merging, the app, the file |
| 9.2 | Built: `GoTo` and `Step`, with `InStep`, applied in place all or none, the gesture taken in when of its source and note; the app sending the tone equalizer's release and nine-band reset as one `Step { source: You }` | engine, app's tone equalizer | other app work |
| 9.3 | Built: merging consecutive changes by target, darktable's 10 s and 2 s, timed from each command's sending and each frame's publication, both injected in tests; a merged change adding no undo record | engine | anything else |
| 9.4 | Built: Edit › Undo and Redo and their keys enabled, naming what they would do, from the mirror; a focused text field keeping its own undo | app | the History tab |
| 8.2 | Built: the clocks' drift bound, `Clock::observe` ignoring another machine's clock read more than 5 minutes past the wall clock (decision 47); version 2's text layer, `sidecar::Edits`: the photo's identity with its fingerprint, `apertrust:copies` with its virtual copies, each its id, number, name, clock vector, log and end, each time an XMP Date; version 1 read as a step of each module, sharing a batch, or as none, end empty, for a file holding nothing; a fixture with a history in `tests/fixtures/xmp` that ExifTool and exiv2 read without error | library | the engine writing version 2 (9.5), virtual copies' commands (step 11), comparing copies (step 10) |
| 9.5 | History saved: the engine reads and writes the log in version 2, through `Edits`; `Edits::parse` the one conversion of version 1 to steps, `Log::read` retired; `VERSION` becoming `COPIES_VERSION`, 2, so `Newer::Version`'s message names the version it compares against; an edit with a `Problem` opened view-only, `ViewOnly` gaining a reason for an unreadable file beside `Newer`, with its own caption, "Some of this edit's history can't be read", not a newer build's; Undo after reopening; the newer file taken in before a write; the crash test; view-only over every entry; the 1,000-step measurements | engine | the app |
| 9.6 | The History tab: lines, numbers, tags, tooltips, a click going to a step, Shift-click and the Default lines opening modules, Reset in the header; `ModuleChange::module` widened to name a module this build does not know | engine, app | the eye, Compress and Truncate |
| 9.7 | The eye: `Hide` and `ModuleState::hidden`, and the tab's eye | engine, app | layers |
| 9.8 | Compress and Truncate: `Compress` and `Truncate`, their undo records, the tab's button with Ctrl-click (Cmd-click on the Mac) and its tooltip; `DiscardHistory` and its undo record, Discard History… in Edit, the photo's right-click menu and the History section's ⋯, asking first with Don't ask again, and allowed on an edit view-only because of a `sidecar::Problem`, writing a fresh log of one step noted `history-discard` | engine, app | snapshots |
| 9.9 | The CLI: `edit` adding a step with `--note`, and `history`, `history --discard` among its moves | cli | MCP |

**Later, with their features:** Auto's steps with the first Auto; pickers' with their pull requests ([pickers.md](pickers.md#the-plan)), source Auto once 9.1 has landed; styles' steps and Create a style… with styles (Phase 3); batches with several photos (decision 37); the assistant's steps with MCP (Phase 6); ratings, labels and metadata as steps (steps 14 and 15); snapshots (step 16); virtual copies' own logs (step 11); diverged logs (step 12); layers' entries and their eye (Phase 4).
