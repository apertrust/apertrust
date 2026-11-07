# 0035. egui, for the desktop and the browser

## Context

Which toolkit draws Apertrust's interface, and can one interface serve both the desktop app and a browser? The photo is drawn by wgpu on the GPU and must appear in the same frame as the controls, with overlays painted on it. The editor needs custom widgets (curves, the tone equalizer's graph, colour wheels, scopes), works well on Linux, Wayland included, and is written in Rust ([0005](0005-rust-and-no-c-or-c-unless-no-rust-option-exists.md)).

## Decision

**egui, through eframe, draws the desktop app.** The photo is drawn by a wgpu paint callback in the same frame as the interface, straight from the engine's display buffer ([0017](0017-threads-and-the-engines-session.md)): no compositing of two technologies, no IPC for pixels, and on-image overlays are more painting. Custom widgets use egui's painter.

**One egui interface serves the desktop app and a desktop browser.** Desktop and browser share mouse, keyboard and a large screen, so they share the interface. Two pieces are pluggable: the engine connection (in process on the desktop, remote over the session protocol in a browser, [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)) and the viewport's source (a GPU display buffer on the desktop, streamed frames in a browser). Platform services sit behind traits so that the browser build can supply its own.

**Two conditions come with egui:**

- a design system before panels are written: tokens, typography and standard components ([0036](0036-the-design-system.md));
- widgets only emit session commands; they never touch engine state.

**Touch is not a layout of this interface.** Phones and tablets get a native mobile app, designed for touch, after Phase 3 ([0001](0001-what-apertrust-is-the-principles.md)).

The accepted costs of a canvas-drawn browser interface: weaker accessibility than the DOM, no native text selection, a download of several megabytes, less integration with the browser.

## Options considered

- Tauri with web panels beside a native viewport: a webview viewport is weak on Linux, where WebKitGTK lacks WebGPU, and the split window is the riskiest piece on Wayland.
- Iced: richer theming and an enforced message architecture, but its web target is not first-class.
- Slint: designer-friendly and touch-ready, but weaker for custom-drawn widgets and viewport integration, and its wasm build is aimed at demos.
- GPUI: polished, but its own renderer shares no wgpu buffers, and it has no web target.
- A separate web client: two interfaces to keep alike, more than a small project can afford.

## Consequences

- The desktop app, `crates/app`, runs eframe; the design system is `apertrust-ui` ([0036](0036-the-design-system.md)).
- Every widget's effect on a photo goes through a session command, so the browser client, scripts and agents see what the app sees.
- The browser build waits for the remote protocol; nothing in the interface may assume the engine is in process.
