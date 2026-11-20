# 0039. One Toolbar, native menus on the Mac, one menu button elsewhere

## Context

Where do the window's title bar and the app's menus live on each platform, and who owns a shortcut that a native menu also shows? The photo should get as much height as possible, the Mac's users expect its native menu bar, and Windows and Linux have no menu bar a frameless window can keep.

## Decision

**The Toolbar runs along the main window's top edge on every platform; it is the window's title bar.** `Toolbar` stays platform-free: told its leading inset and whether to draw caption buttons, it reports drags and double-clicks on its empty areas, and right-clicks anywhere on it, which open the toolbar menu on every platform. It is a toolbar a person customises, its items a list kept for each workspace ([0092](0092-the-toolbar.md), planned).

- **macOS:** a full-size content view with a transparent title bar and a hidden title. The traffic lights stay AppKit's buttons, placed in the Toolbar by `apertrust-display` after every layout AppKit makes, which reports the room they take; the Toolbar's first item after them is Import…. A double-click follows the system's setting; the title stays set for Mission Control, the Window menu and VoiceOver.
- **Windows:** decorations off, so Win+arrows, Alt+Space and Alt+F4 stay the system's; minimise, maximise and close drawn by the app at the bar's end, each an AccessKit button. A drag on empty areas starts the system's move with Aero Snap, a double-click maximises, bands inside the edges resize; the system menu is on Alt+Space, a right-click on the bar opening the toolbar menu ([0092](0092-the-toolbar.md)). Native hit testing from `apertrust-display` (`WM_NCHITTEST`) brings back Windows 11's snap layouts flyout (planned).
- **Linux, X11 and Wayland:** decorations off, caption buttons at the end, the move and resize handed to the window manager or compositor, a double-click maximising, the compositor's window menu on its own key (Alt+Space on GNOME), none on X11, a right-click on the bar opening the toolbar menu ([0092](0092-the-toolbar.md)), 6-point bands inside the edges resizing.
- **Use the system's title bar,** a setting on Windows and Linux, off by default, turns decorations back on.
- **The minimum window size** is 500 by 600 points, so Windows 11's half and third snap zones take the window; the shell gives way below 960 points.

**One menu model,** `MENUS`, a static tree in the app naming action ids, tied to the actions registry by tests ([0036](0036-the-design-system.md)). Each item's words, key, enabled state and check mark come from the registry, the keymap and the frame's state. Its menus, in order: the app menu (the Mac's), File, Edit, Photo, Select (the selection's items, darktable's select tool), Layer (from Phase 4), View, Window (the Mac's) and Help, the same on every platform but for the Mac's own.

- **On the Mac** muda builds the native menu bar in place of winit's default, with AppKit's own items (Services, Hide, Minimise, Zoom, the window list, Help's search) and every other item a registry action; AppKit's own full-screen item is kept out. Only what changed is sent each frame; a choice comes back through a channel and runs through `Shell::run`. The app menu is titled Apertrust by an Info.plist embedded in the binary.
- **On Windows and Linux** one menu button sits at the Toolbar's very start by default, never folded while shown: a button named Menu that opens `MENUS` as one drawn pull-down of submenus through the popup service ([0040](0040-popup-menus-native-on-the-mac-drawn-elsewhere.md)). It is drawn as the app's icon, the macOS icon's rounded square in one colour with the Slider A cut out of it, then a caret, 20 points in a 30-point-high button; or as ☰. The setting Menu button (`toolbar.menu_button`, Settings › General › Interface, Windows and Linux only, applied at once) chooses: Apertrust icon, the default, or Menu icon. Either way its name, its popup, its tooltip, "Menu (F10)", and its keys are the same. F10 opens it; on Windows, Alt pressed and released alone too. A top-level menu's access key, the `&` in its message, opens it at that menu with Alt (Alt+F, Alt+E). It is the first control by default, and F6 lands on the toolbar's first control. A person may hide it ([0092](0092-the-toolbar.md)): F10, Alt alone on Windows and Alt with an access key still open the menus, hung from the toolbar's start.
- **About** is a menu item on every platform, in the app menu on the Mac and in Help on Windows and Linux; the Toolbar has no mark of its own that opens it.

**The registry owns every key.** The native menu shows the registry's keys as key equivalents and runs its actions. While a widget has the focus, the menus hold none of the keys it keeps, except Undo, Redo, Cut, Copy, Paste and Select All, which act on a focused text field; without one, Undo and Redo act on the photo and Cut, Copy and Paste are disabled. Rebinding updates the menu. The platforms' menu keys, and on Windows and Linux Alt with a letter, are refused for actions. Hold actions are never menu items.

## Options considered

- The system's frame above the Toolbar: a second bar's height taken from the photo.
- Client-side decorations only where the compositor draws none: KDE and X11 would show a second title bar.
- muda's Win32 menu bar: it lives in the non-client area the integrated frame removes.
- egui menus on the Mac, or the Mac's menus written by hand in FFI.
- Menu titles in a row of the Toolbar on Windows and Linux, or a row of their own: room taken from the photo and from the bar's actions.
- ☰ with the app's mark beside it as a button of its own that opened About: two items at the bar's start, the mark doing only what Help › About already does, and on the Mac what the app menu's About does.
- ☰ at the bar's end, far from the actions it lists.
- Menus that own their keys: two meanings for one key.

## Consequences

- `apertrust-display` holds the Mac's traffic-light placement, the full-screen item's removal and Windows' hit testing; muda is a macOS-only dependency.
- Every menu item is a registry action, so the menu button, the Mac's bar and the shortcuts cannot disagree.
- `apertrust-ui` draws the menu button in either style (`Toolbar::menu_button_style`, `MenuButtonStyle`), the app's icon from its own SVG, `app-icon.svg`, tinted like the other icons; the app passes the setting each frame.
- Planned: Windows' Alt pressed and released alone and native hit testing; Linux's button layout from the settings portal; X11's window menu; the toolbar and its menu ([0092](0092-the-toolbar.md)).
