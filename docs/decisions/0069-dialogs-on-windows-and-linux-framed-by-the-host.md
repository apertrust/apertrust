# 0069. Dialogs on Windows and Linux, framed by the host

## Context

On Linux, Export was disabled until the window manager's modal hints and Linux's alerts were built ([0041](0041-dialogs-as-native-modal-windows.md)), though Settings already showed there as a modal window. GNOME on Wayland draws no frame round a window, so every dialog window showed winit's fallback frame: minimise, maximise and close as bare squares, though a dialog can only be closed. On Wayland the window also kept the host's first guess at its size, so Export showed cut off.

On Windows the main window's top panel is its title bar ([0039](0039-one-top-bar-native-menus-on-the-mac-one-menu-button-elsewhere.md)), drawn in the theme; the system's title bar on a dialog would show an accent-coloured caption and the app's icon beside it.

## Decision

**Dialogs show on Linux as the host's modal windows now, before the window manager's hints, and the host draws their frame on Windows and Linux.** This narrows [0041](0041-dialogs-as-native-modal-windows.md)'s system frame to the Mac.

- **Export on every platform.** File › Export… is enabled with a photo open on the Mac, Windows and Linux: a sheet on the Mac, a modal window elsewhere, as the host gives each form (`dialog_host::platform::form`). Nothing outside the host knows the platform.
- **What the host can ask.** `platform::shows_alerts` says whether the host can show an alert: on the Mac, `NSAlert`, and on Linux, the alert the host draws (`platform::draws_alerts`), not on Windows until its task dialog is built. Where it cannot, Export lists On conflict's Overwrite disabled, with its reason in the tooltip, and rests while a kept Overwrite is chosen.
- **The frame on Windows and Linux** (`platform::draws_frame`): the dialog's window has no decorations, on X11 too, so the frame is the same on every desktop. On Windows it keeps the system's undecorated shadow and Windows 11's rounded corners, as the main window does (`with_undecorated_shadow`). The host draws a 32-point strip at the window's top, in the dialog's own colour, with one close button at its end in the main window's caption style (`components::caption_button`), which closes the dialog as its Cancel, and the window's title centred on it, kept clear of the close. There is no icon. A drag on the strip's empty part, the title included, moves the window through the system, the compositor or the window manager, and on Linux a right-click opens the window menu. On Windows a right-click does nothing: winit's own window menu there enables Minimise always and Maximise for a window the person resizes, neither of which a dialog has. The dialogs draw nothing of it.
- **The title.** A dialog's title belongs to its window's frame, the Mac's title bar or this strip, and its content draws one only where the frame has none, the Mac's sheet: Export's header shows there alone. An alert's strip shows no title, since its body starts with it, as every platform's alerts do. The strip's title is only painted: it takes no press, and it adds no node for screen readers, where the window is named by its title already.
- **The size.** A window the person cannot resize is held at its size by its minimum and maximum, which Wayland's winit sets to the first guess as the window is made; the host moves both with each resize to the content's size. A window the person resizes resizes from bands inside its edges, as the main window does.

## Options considered

- Waiting for the window manager's modal hints before showing Export: the host's modal window already keeps the main window's input and brings itself forward on a click.
- Each dialog drawing its own title on Linux: a second rule beside the Mac's title bars, and a header Settings and the jobs and log windows would add only there.
- winit's Adwaita frame without its title (`wayland-csd-adwaita-notitle`): five more crates, a second bar above the dialog's own header with no title, minimise and maximise possibly shown, and Wayland only.
- A close button over the dialog's top corner, with no strip: it would cover content that reaches the top, as the jobs window's rows do.
- On Windows, the system's title bar coloured to the theme through DWM's caption and text colours: `unsafe` FFI in `apertrust-display`, Windows 11 only, and the icon and the system's caption buttons still there.

## Consequences

- Settings on Windows and Linux has the frame's close and its own Close.
- Export draws its header with its title in the Mac's sheet alone, so on Windows and Linux its title shows once, in the frame.
- Wayland's `xdg_toplevel.set_parent` and Linux's drawn alert are built, so Overwrite is offered on Linux; X11's `WM_TRANSIENT_FOR` and `_NET_WM_STATE_MODAL` stay to build.
- On Wayland the host also draws each dialog window's border and drop shadow round a transparent margin ([0041](0041-dialogs-as-native-modal-windows.md)); X11 has none, and Windows has the system's undecorated shadow.
