# 0058. A native engine, every client through the session protocol

## Context

Apertrust should be usable from the desktop app, the command line, a browser or tablet at home or over the Internet, AI agents and scripts. Running the engine in a browser would mean uneven WebGPU on Linux, buffer limits, threads behind cross-origin isolation and smaller AI models. How do all these clients reach one engine, and where does the engine run?

## Decision

**The engine always runs natively,** on the person's machine or a home server. Browsers and tablets are remote clients, on the local network or over the Internet. There is no engine in the browser.

**Every client reaches the engine through one session protocol** ([session-protocol.md](../session-protocol.md)): the desktop app, `apertrust-cli`, and later the web client, MCP agents and scripts. A client sends commands and receives events ([0059](0059-one-event-system.md)); it never calls into the engine's state. The engine never depends on UI code, and widgets only emit commands, so undo, agents and the remote mode behave as the app does.

- **Clients speak modules, not operations** ([0004](0004-tools-for-people-operations-for-algorithms.md)): controls by module and control id, the session turning them into operation parameters.
- **Commands are plain data,** numbered in the order sent and coalesced into batches on the engine thread ([0017](0017-threads-and-the-engines-session.md)); a refused command names itself.
- **The library's commands are the session's too** ([0054](0054-four-layers-for-the-librarys-code.md)), so a browser client's library lives on the server.
- **The types stay in the engine** (`apertrust_engine::session`'s `Command` and `Event`) until a client must compile them without it; then they become `apertrust-protocol`.

**One server, planned:** `apertrust-cli serve`, the `apertrust-server` library crate ([0006](0006-two-programs-on-three-desktop-platforms.md)). It first syncs the desktop app's photos with a NAS ([0071](0071-a-self-hosted-server-with-sync.md)), through the library's storage protocol rather than this one, and serving the engine to browser and tablet clients through this protocol is a separate, later option for the same program.

**Remote access, planned,** has requirements a local app does not:

- authentication and encrypted transport (HTTPS, secure WebSocket or WebTransport over TLS), a VPN the recommended default over the Internet;
- frames streamed by bandwidth: compressed at reduced resolution while a slider moves, full quality when idle, wide-gamut and HDR frames in a codec that keeps them;
- the photo library on the server, the client only browsing and editing it;
- notices queued per connection and facts coalesced, a late client synced, one protocol number ([0059](0059-one-event-system.md)).

**MCP, planned,** runs through `rmcp` as a thin client of the session protocol: listing images, reading modules and their controls, setting values, histograms and previews, which MCP tools can return as images. It connects only when the person allows it, read-only unless writes are allowed, and what it changes is ordinary history (principle 8).

**Scripts** reach the engine the same way, through the session protocol and MCP, from any language. Apertrust embeds no scripting language for now; an embedded one (Lua through `mlua`, or Rhai) is reconsidered if users ask.

**Kept cheaply possible,** should an engine in the browser ever be wanted: pure-Rust dependencies where choices are close, tiling within browser buffer limits, an engine API that never blocks its caller.

## Options considered

- Processing in the browser first: a reduced engine on uneven WebGPU, with smaller AI models.
- A separate API for each client (a library for the CLI, another for MCP): clients would drift apart and each would need its own undo.
- Clients that speak operations: they would each translate module values and could set what no module owns.

## Consequences

- The CLI runs the same session in process (`Session::start`, `send`, `events`), so its exports and edits are the app's.
- A new feature is a command and its events first; the app's panels draw from what the session reports.
