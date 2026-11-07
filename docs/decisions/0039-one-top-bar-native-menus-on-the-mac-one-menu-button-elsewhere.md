# 0039. One top bar, native menus on the Mac, one menu button elsewhere

## Context

Where do the window's title bar and the app's menus live on each platform, and who owns a shortcut that a native menu also shows? The photo should get as much height as possible, the Mac's users expect its native menu bar, and Windows and Linux have no menu bar a frameless window can keep.

## Decision

**One top bar runs from the window's top edge on every platform; it is the window's title bar.** `TopPanel` stays platform-free: told its leading inset and whether to draw caption buttons, it reports drags, double-clicks and right-clicks on its empty areas.

- **macOS:** a full-size content view with a transparent title bar and a hidden title. The traffic lights stay AppKit's buttons, placed in the top bar by `apertrust-display` after every layout AppKit makes, which reports the room they take. A double-click follows the system's setting; the title stays set for Mission Control, the Window menu and VoiceOver.
- **Windows:** decorations off, so Win+arrows, Alt+Space and Alt+F4 stay the system's; minimise, maximise and close drawn by the app at the bar's end, each an AccessKit button. A drag on empty areas starts the system's move with Aero Snap, a double-click maximises, a right-click opens the system menu, bands inside the edges resize. Native hit testing from `apertrust-display` (`WM_NCHITTEST`) brings back Windows 11's snap layouts flyout (planned).
- **Linux, X11 and Wayland:** decorations off, caption buttons at the end, the move and resize handed to the window manager or compositor, a double-click maximising, the compositor's window menu on Wayland, 6-point bands inside the edges resizing.
- **Use the system's title bar,** a setting on Windows and Linux, off by default, turns decorations back on.
- **The minimum window size** is 500 by 600 points, so Windows 11's half and third snap zones take the window; the shell gives way below 960 points.

**One menu model,** `MENUS`, a static tree in the app naming action ids, tied to the actions registry by tests ([0036](0036-the-design-system.md)). Each item's words, key, enabled state and check mark come from the registry, the keymap and the frame's state.

- **On the Mac** muda builds the native menu bar in place of winit's default, with AppKit's own items (Services, Hide, Minimise, Zoom, the window list, Help's search) and every other item a registry action; AppKit's own full-screen item is kept out. Only what changed is sent each frame; a choice comes back through a channel and runs through `Shell::run`. The app menu is titled Apertrust by an Info.plist embedded in the binary.
- **On Windows and Linux** one menu button, ☰, sits at the top bar's very start, before the mark, never folded: an icon button named Menu that opens `MENUS` as one drawn pull-down of submenus through the popup service ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)). F10 opens it; on Windows, Alt pressed and released alone too. A top-level menu's access key, the `&` in its message, opens it at that menu with Alt (Alt+F, Alt+E). It is the first control, so F6 lands on it.

**The registry owns every key.** The native menu shows the registry's keys as key equivalents and runs its actions. While a widget has the focus, the menus hold none of the keys it keeps, except Undo, Redo, Cut, Copy, Paste and Select All, which act on a focused text field; without one, Undo and Redo act on the photo and Cut, Copy and Paste are disabled. Rebinding updates the menu. The platforms' menu keys, and on Windows and Linux Alt with a letter, are refused for actions. Hold actions are never menu items.

## Options considered

- The system's frame above the top bar: a second bar's height taken from the photo.
- Client-side decorations only where the compositor draws none: KDE and X11 would show a second title bar.
- muda's Win32 menu bar: it lives in the non-client area the integrated frame removes.
- egui menus on the Mac, or the Mac's menus written by hand in FFI.
- Menu titles in a row of the top bar on Windows and Linux, or a row of their own: room taken from the photo and from the bar's actions.
- The mark as the menu (About hidden one step down), or ☰ at the bar's end, far from the actions it lists.
- Menus that own their keys: two meanings for one key.

## Consequences

- `apertrust-display` holds the Mac's traffic-light placement, the full-screen item's removal and Windows' hit testing; muda is a macOS-only dependency.
- Every menu item is a registry action, so the ☰, the Mac's bar and the shortcuts cannot disagree.
- Planned: Windows' Alt pressed and released alone and native hit testing; Linux's button layout from the settings portal; X11's window menu.
