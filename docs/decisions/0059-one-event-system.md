# 0059. One event system

## Context

The session's threads (the engine thread, the edit writer, the jobs, the library's work) all report to clients: the app now, later the server, the web client and MCP. darktable uses one global bus of signals whose handlers run inside the raise on the GUI thread; that has brought it deadlocks, signal storms (thousands of signals for one batch), an order no one guarantees and handlers outliving their data. How do events flow in Apertrust, so that every client can tell what is current and what may be dropped?

## Decision

**One event system for every layer: two kinds of event, numbered against commands, sent through one outbox per session, entering one reducer per client; no bus** ([events.md](../events.md)).

1. **Facts and notices** (`Event::topic()`): a fact states its topic's whole current state and supersedes the last of its topic (the values, a frame, a scopes reading, the Export dialog's preview, a job's fraction, message and state by job; later a readout by module, the selection, a page shown); a notice happens once (opened, saved, failed, a job added or ended, an exported photo, the library's listing chunks and deltas, a notice for the person).
2. **Numbers:** `through` on values, frames, scopes, saves and previews, never decreasing; the open's number on everything about an opened photo or folder; a job's command on its `Added`; a refused command names itself, `SessionError::Refused { command, error }`. A client shows a value it sent until a `Values` includes it, so a slider never jumps back.
3. **One `Outbox` per session** (`session/outbox.rs`), used by every thread of the session, calling `wake` once per burst, a flag cleared as `events()` drains with one loop of `try_recv`. Lower crates take an observer closure, as base's `Control::start` does, which the session wraps into `Event::Job` and `Event::Library`.
4. **No send blocks and no producer waits on a client:** a question is a notice and an answering command, waited through `Context::wait`.
5. **Order:** one producer's events in order; in a batch a refusal before the values, the values before the frame, the scopes after it; across producers only numbers order.
6. **In process** every event arrives exactly once; nothing is sent after the handle drops.
7. **One event per command or batch,** never per item, carrying what a client shows; a collection's photos by page. The one exception is a thumbnail: one event per photo, since each is its own job and carries its pixels.
8. **Remote, planned:** per connection, notices queued to 1024 and facts coalesced by topic, a lagging connection closed and resynced; `Hello` and `Welcome` with one protocol number, raised only by a change a client cannot ignore; JSON commands and events with binary frames; `Sync` for a late client; numbers one sequence across connections; a second in-process client, the app's MCP endpoint, with a receiver of its own.
9. **In the app one reducer:** `crates/app/src/mirror.rs`'s `apply(event) -> Vec<Reaction>`, free of egui, drained first in `App::logic` (so while the window is minimised too); panels draw from it each frame; one-shot reactions (the jobs window, a live region, a toast) run through `Shell::run`. The app's own changes (theme, settings, tab, hover) are its state, with no event. It is tested by feeding it event sequences.
10. **`-d event`** (`signal` an alias) logs each send and drain, and a count per kind each second under `verbose`.

**Planned:** `Event::Notice { message, photo }` for the person, its place in the interface decided with it.

**Where it differs from darktable:** no bus, no raise inside a raise, no synchronous signal, numbers in place of blocking a handler round its own raise, events carrying their data, no per-widget handlers, one event per batch, a stated order, notices as message ids, a repaint request for every redraw.

## Options considered

- darktable's publish–subscribe bus: its problems come with it.
- Leaving events unclassified: the server and MCP would each decide what may be dropped.
- An Elm or Redux store for the whole app: it would rewrite the actions registry and duplicate the engine's undo.
- State synchronisation by diffs or a CRDT: frames do not fit it.

## Consequences

- A new event says its kind and its number; a new producer sends through the outbox.
- The remote protocol needs no new classification: facts coalesce, notices queue.
