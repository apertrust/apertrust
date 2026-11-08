# 0006. Two programs on three desktop platforms

## Context

Which platforms does Apertrust run on, and as which programs? Photographers edit on Windows, macOS and Linux, and batch work, servers and agents want a program with no window. A separate program for every role multiplies packaging and versions.

## Decision

**Windows, macOS and Linux, first-class:** the same features and the same results on each ([0001](0001-what-apertrust-is-the-principles.md)). Linux covers Wayland and X11.

**Two executables:**

| Program | Crate | Role |
| --- | --- | --- |
| `apertrust` | `crates/app` | The desktop app: egui on the engine's session ([0035](0035-egui-for-the-desktop-and-the-browser.md)) |
| `apertrust-cli` | `crates/cli` | Headless: `export`, `edit`, `info`, `preview`, `gpu` and `library` now; `batch`, `serve` and `mcp` planned |

**The server and MCP are libraries inside these programs,** not programs of their own: `apertrust-cli serve` and `apertrust-cli mcp` run them, and the app may offer an MCP endpoint ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

Both programs take the same debug domains, `--version`, user data folder flags and `--conf` overrides ([0043](0043-settings-and-apertrusts-folders.md), [0044](0044-logging-the-banner-and-versions.md)).

A native mobile app is a later phase ([0001](0001-what-apertrust-is-the-principles.md)); a browser reaches a native engine as a remote client, never running the engine itself ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

## Options considered

- Separate server and MCP programs: more binaries to package and version, for code the CLI already links.
- One program with a headless mode: a CLI that pulls in the window toolkit and its platform libraries.
- Tauri with web panels beside a native wgpu viewport: WebKitGTK lacks WebGPU, and the split window is fragile on Wayland ([0035](0035-egui-for-the-desktop-and-the-browser.md)).

## Consequences

- The engine stays free of UI code, so both programs drive the same session.
- CI tests on all three platforms: Linux on lavapipe for every pull request, Windows on WARP and macOS on Metal every night and on pull requests that need them ([0070](0070-ci-in-three-layers.md)).
- Each binary computes its own version in its own build, so a commit rebuilds no library ([0044](0044-logging-the-banner-and-versions.md)).
