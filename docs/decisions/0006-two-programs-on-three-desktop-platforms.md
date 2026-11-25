# 0006. Three programs on three desktop platforms

## Context

Which platforms does Apertrust run on, and as which programs? Photographers edit on Windows, macOS and Linux, and batch work, servers and agents want a program with no window. A separate program for every role multiplies packaging and versions.

## Decision

**Windows, macOS and Linux, first-class:** the same features and the same results on each ([0001](0001-what-apertrust-is-the-principles.md)). Linux covers Wayland and X11.

**Three executables:**

| Program | Crate | Role |
| --- | --- | --- |
| `apertrust` | `crates/app` | The desktop app: egui on the engine's session ([0035](0035-egui-for-the-desktop-and-the-browser.md)) |
| `apertrust-cli` | `crates/cli` | Headless: `export`, `edit`, `info`, `preview`, `gpu` and `library` now; `batch` and `serve` planned |
| `apertrust-mcp` | `crates/mcp` | Planned: the MCP server agents launch, over stdio, joining the running app's session or, with `--headless`, running one of its own ([0108](0108-agents-through-apertrust-mcp-joining-the-apps-session.md)) |

**The server is a library inside the CLI,** not a program of its own: `apertrust-cli serve` runs it ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)). **MCP is a program of its own,** since an MCP client launches a command and reads its stdout as JSON-RPC alone; the app never speaks MCP ([0108](0108-agents-through-apertrust-mcp-joining-the-apps-session.md)).

Every program takes the same debug domains, `--version`, user data folder flags and `--conf` overrides ([0043](0043-settings-and-apertrusts-folders.md), [0044](0044-logging-the-banner-and-versions.md)).

Touch is a later phase: the app's interface in Touch mode on tablets, and a phone's layout ([0001](0001-what-apertrust-is-the-principles.md), [0094](0094-one-interface-standard-and-touch-modes.md)); a browser reaches a native engine as a remote client, never running the engine itself ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

## Options considered

- A separate server program: one more binary to package and version, for code the CLI would link anyway.
- MCP as `apertrust-cli mcp`: its stdout shared with a CLI's subcommands, and a client's configuration naming a subcommand of a program that does something else ([0108](0108-agents-through-apertrust-mcp-joining-the-apps-session.md)).
- One program with a headless mode: a CLI that pulls in the window toolkit and its platform libraries.
- Tauri with web panels beside a native wgpu viewport: WebKitGTK lacks WebGPU, and the split window is fragile on Wayland ([0035](0035-egui-for-the-desktop-and-the-browser.md)).

## Consequences

- The engine stays free of UI code, so every program drives the same session.
- CI tests on all three platforms: Linux on lavapipe for every pull request, Windows on WARP and macOS on Metal every night and on pull requests that need them ([0070](0070-ci-in-three-layers.md)).
- Each binary computes its own version in its own build, so a commit rebuilds no library ([0044](0044-logging-the-banner-and-versions.md)).
