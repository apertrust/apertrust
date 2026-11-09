# 0069. Dialogs on Linux, framed by the host

## Context

On Linux, Export was disabled until the window manager's modal hints and Linux's alerts were built ([0041](0041-dialogs-as-native-modal-windows.md)), though Settings already showed there as a modal window. GNOME on Wayland draws no frame round a window, so every dialog window showed winit's fallback frame: minimise, maximise and close as bare squares, though a dialog can only be closed. On Wayland the window also kept the host's first guess at its size, so Export showed cut off.

## Decision

**Dialogs show on Linux as the host's modal windows now, before the window manager's hints, and the host draws their frame there.** This narrows [0041](0041-dialogs-as-native-modal-windows.md)'s system frame for Linux.

- **Export on every platform.** File › Export… is enabled with a photo open on the Mac, Windows and Linux: a sheet on the Mac, a modal window elsewhere, as the host gives each form (`dialog_host::platform::form`). Nothing outside the host knows the platform.
- **What the host can ask.** `platform::shows_alerts` says whether the host can show an alert: on the Mac, `NSAlert`, and on Linux, the alert the host draws (`platform::draws_alerts`), not on Windows until its task dialog is built. Where it cannot, Export lists On conflict's Overwrite disabled, with its reason in the tooltip, and rests while a kept Overwrite is chosen.
- **The frame on Linux** (`platform::draws_frame`): the dialog's window has no decorations, on X11 too, so the frame is the same on every desktop. The host draws a 32-point strip at the window's top, in the dialog's own colour, with one close button at its end in the main window's caption style (`components::caption_button`), which closes the dialog as its Cancel. A drag on the strip's empty part moves the window through the compositor or window manager, and a right-click opens the window menu. The dialogs draw nothing of it.
- **The size.** A window the person cannot resize is held at its size by its minimum and maximum, which Wayland's winit sets to the first guess as the window is made; the host moves both with each resize to the content's size.

## Options considered

- Waiting for the window manager's modal hints before showing Export: the host's modal window already keeps the main window's input and brings itself forward on a click.
- winit's Adwaita frame without its title (`wayland-csd-adwaita-notitle`): five more crates, a second bar above the dialog's own header with no title, minimise and maximise possibly shown, and Wayland only.
- A close button over the dialog's top corner, with no strip: it would cover content that reaches the top, as the jobs window's rows do.

## Consequences

- Settings on Linux has the frame's close and its own Close, as on Windows, where the system's frame has a close beside it.
- Wayland's `xdg_toplevel.set_parent` and Linux's drawn alert are built, so Overwrite is offered on Linux; X11's `WM_TRANSIENT_FOR` and `_NET_WM_STATE_MODAL` stay to build.
- On Wayland the host also draws each dialog window's border and drop shadow round a transparent margin ([0041](0041-dialogs-as-native-modal-windows.md)); X11 has none.
