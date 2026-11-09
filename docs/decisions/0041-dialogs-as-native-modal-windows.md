# 0041. Dialogs as native modal windows

## Context

How are dialogs (Settings, Export, Customise tabs, prompts) and confirmations shown? A dialog should behave as the platform's own: a window modal to the one it rises from, a sheet on the Mac, reachable by screen readers. egui's viewports in eframe 0.37 hand the app no window to make owned, transient or a sheet, and only the root window gets an AccessKit adapter.

## Decision

**Every dialog is a native window of its own, modal to the window it rises from, its content drawn by Apertrust's components. Every confirmation is the system's alert. About, and a future splash screen while the app loads, stay Apertrust's own, drawn over the window on the `scrim` token.**

**The dialog host** (`crates/app/src/dialog_host.rs`): the app hosts dialog windows beside eframe's. `Gated`, which holds the submit gate round every eframe callback ([0018](0018-one-submit-gate-for-the-engine-and-the-window.md)), creates each dialog's winit window when asked, with its own egui context (the app's fonts, theme, language and scale), egui-winit state and AccessKit adapter, made before the window first shows, and a surface and egui-wgpu painter on the engine's one device. The window's events, AccessKit requests and repaints are routed to the host, never to eframe. Each dialog is one type implementing `Dialog` (its id, title, form, keys and content); its content is a function of `&mut Ui`, so a browser can draw it over the page and tests in a harness. One dialog at a time.

**Modality by platform:**

| Platform | Form |
| --- | --- |
| macOS | A sheet on the window, AppKit's own, centred with the window dimmed behind it, begun and ended from the run loop outside winit's handler (`begin_sheet`, `end_sheet` in `apertrust-display`) |
| Windows | An owned window, its owner disabled while it is open |
| X11 | The dialog window type; `WM_TRANSIENT_FOR` and `_NET_WM_STATE_MODAL` (planned) |
| Wayland | `xdg_toplevel.set_parent`; no xdg-dialog modal hint, whose attached presentation on Mutter keeps the dialog from moving independently |

Everywhere the app also takes the main window's input while a modal dialog is open. On Wayland every hosted window, modal or modeless, has a hairline border and a drop shadow around a transparent outer margin, rather than dimming the main window; X11 has none, since the margin can show black without a compositing manager; Windows' dialogs will have them once they are undecorated on a transparent surface (planned), keeping the system's frame and shadow until then; on macOS AppKit supplies the presentation.

**Settings is the one modeless dialog,** a window of its own: on the Mac no Close button, Escape doing nothing, Cmd+W (File › Close Window) closing it, minimise and zoom dimmed; it opens where it was last left, kept in settings and restored on its monitor, centred on the main window the first time. While it has the focus, the actions on the main window or its photo rest. The jobs window ([0057](0057-jobs-as-darktables-control-jobs.md)) is a second modeless window beside it.

**Keys and focus:** a dialog opens with the focus in it and gives it back to the control that opened it; Escape cancels (Cmd+. too on the Mac); Enter presses the default button unless the focused control takes Enter; Tab and F6 stay inside; while a modal dialog is open the view's keys and every action that opens a dialog rest. Quit, or closing the main window, closes the dialog as its Cancel first. A dialog's buttons sit at the row's end in the platform's order. A dialog opens centred on its parent, sized by its design and fixed at that size, unless it is a window to work beside the photo and resizes with a least size its content fits, as the log window does (its size kept with its place); each window takes its monitor's scale times the interface's.

**Alerts:** plain requests (`alert::Request`: a title, a message, buttons with their role, Default, Cancel or Destructive, and an optional suppression box), answered on a later frame. On the Mac an `NSAlert` as a sheet, the destructive button marked, every key set (`begin_alert`); on Windows the task dialog, owned by the window, on a thread of its own (planned); on Linux, which has no system alert for an app outside GTK and Qt, Apertrust's own drawn alert, `components::AlertBody`, in a dialog window, in the theme, on every desktop. The words are Fluent messages, never the system's OK and Cancel.

**Print and Import are not dialogs:** they are full-window sheets inside the main window ([0037](0037-one-view-the-library-as-a-tab.md)), since each takes the window's whole size for its preview or its thumbnails and is a place to work.

## Options considered

- egui's immediate or deferred viewports, finished in `apertrust-display`: no window handle to finish them by, and silent to screen readers.
- A patched eframe: a fork to carry.
- Dialogs written in each native toolkit: three interfaces, and none on Linux.
- Every dialog an egui modal over the window: not a system window.
- rfd's message dialogs, or zenity and kdialog on Linux: no destructive button or Escape on a translated Cancel, unparented, often missing.
- A drawn alert on every platform: not the system's.

## Consequences

- `apertrust-display` holds the sheets, alerts and window flags each platform needs; the app's dialogs never touch them directly.
- The submit gate covers dialog surfaces, which are configured inside `Gated`'s held callbacks.
- Revisit once eframe gives every viewport an AccessKit adapter and the app its window: the content functions move unchanged.
