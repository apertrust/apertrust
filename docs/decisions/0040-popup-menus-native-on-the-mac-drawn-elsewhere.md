# 0040. Popup menus: native on the Mac, drawn elsewhere

## Context

Dropdowns, pull-downs and context menus appear everywhere in the interface. On the Mac, beside a native menu bar, people expect native popups; elsewhere a native popup would look foreign beside drawn menus. egui's own menus open submenus with no pause, read no typed text and take no keys from the rest of the frame, so a letter typed in a menu would also run the registry's action for it.

## Decision

**One popup service that every component asks; native lists on the Mac, drawn popups everywhere else.**

**The service** (`apertrust_ui::popup`) takes plain requests (a kind, an anchor, a name for screen readers, entries: items with an id, words, keys, a check mark and enabled; separators; submenus) and gives back an answer, the id chosen or none, on a later frame, as a click arrives. A request carries its opener, the control the focus goes back to. An app installs a `popup::Service` in a context (`popup::install`) to show the requests it takes itself, answering through `popup::answered`; the drawn popups show the rest, and everything where none is installed.

**On the Mac** the app's service hands every plain list to `apertrust_display::pop_up_menu`: dropdowns for a value (`Choice`, the chosen item placed over the control, with AppKit's type-to-jump), pull-downs from a button and context menus. The display crate builds an `NSMenu` from plain data of its own, `autoenablesItems` off, every item's target an object that only notes the tag chosen, and shows it with `popUpMenuPositioningItem:atLocation:inView:` from the main run loop, outside winit's event handler, so the window does not freeze; the menu's appearance follows the theme, the font the system's times the interface's scale.

**Drawn everywhere:** every popup on Windows, Linux and in the browser, and on the Mac whatever previews on hover (presets, styles), holds controls, colour, thumbnails or a search field, and tooltips.

**How the drawn popups work:**

- each open level is an egui `Area`, drawn from the tokens by `popup::show`, which the app calls once a frame; a popup the last pass neither opened nor drew closes, answering Dismissed;
- an egui plugin takes the last popup's keys out of each frame's input before egui reads it: every press without Ctrl, Cmd or Alt except the F-keys, and all typed text; keys with a modifier and F-keys stay the app's, so F10 closes the ☰'s menu;
- opening moves the focus to the chosen line, closing by keys or a choice gives it back; a press outside closes every level and is not swallowed;
- a submenu opens once the pointer has rested 0.2 s on its line (`PAUSE`);
- the drawn dropdown filters as darktable's does while letters are typed;
- an answer lasts the frame it arrives in and is gone once read (`popup::answer`).

**The slider's fine-control popup** is a kind of the service drawn on every platform ([0042](0042-numbers-shown-typed-and-stored.md)).

## Options considered

- Drawn everywhere: not what the Mac's users expect beside a native menu bar.
- Win32 popups on Windows: dark only through undocumented calls, without the interface's scale or the high-contrast theme.
- muda's context menus: `unsafe`, no item placed over the control, one global channel.
- egui's `Popup` and menus for the drawn ones: no type-ahead, no pause, no key ownership without reaching into egui's private state.
- Keys taken by a call each client makes: a client that forgets leaks keys to the registry.

## Consequences

- Every component that opens a list goes through the service; none draws its own.
- `apertrust-display` owns the Mac's menus; `apertrust-ui` owns the drawn ones.
- Open: whether the window repaints while a native popup tracks; hover previews in native menus through `NSMenuDelegate` if it does.
