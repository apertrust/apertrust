# 0094. One interface, Standard and Touch modes

## Context

Apertrust's interface is drawn for a pointer and a keyboard: rows 26 points high, sliders dragged by their track, presets previewed on hover, context menus on a right-click. Tablets need the same editor under a finger. The question is whether touch gets an interface of its own or a mode of the one interface, what the setting is called and what it defaults to, and what interface code must do now, while only the pointer's mode is built, so that touch can be added later without rewriting panels.

What constrains it:

- One egui interface serves the desktop and the browser ([0035](0035-egui-for-the-desktop-and-the-browser.md)); a second interface is more than a small project can keep in step.
- Every command is a registry action, and a component reads its sizes from the theme's tokens ([0036](0036-the-design-system.md)).
- Context menus open only in named places, each item a registry action ([0091](0091-context-menus-places-and-tools-items-as-actions.md)).
- Every client reaches the engine through the session protocol ([0058](0058-a-native-engine-every-client-through-the-session-protocol.md)).

## Decision

**One interface with two modes, Standard and Touch.** The tabs, the tools and their sections, the rows, the actions registry, the selection model and the session's commands are the same in both. Touch mode changes sizes, spacing and gestures, and makes two named layout adaptations that present the same parts: the side panel as a bottom sheet in portrait, and the Photos tab's Select mode with its batch bar. The dividing line is the input model, not the platform. Touch mode is the tablet's interface; a phone's layout is a later, separate design. Standard is built; Touch mode is planned, its design on the wireframe's Tablet page.

### The setting

- **Settings › General › Interface:** Interface, a choice of Automatic, Standard and Touch (`interface.mode`, planned: `automatic`, `standard`, `touch`, applied at once).
- **View › Touch Mode,** checked while on, is the quick switch (`touch-mode`, planned). It sets the setting to Touch, or to Standard when Touch is on.
- **Automatic** picks Touch on a device with a touch screen and no pointer, Standard otherwise.
- **The defaults:** Standard on Windows, macOS and Linux, where Automatic is offered as a choice; Touch on Android.

### Touch mode's rules

Planned, as the wireframe's Tablet page draws them:

- **Targets** at least 44 points; rows about 1.7 times as tall as Standard's.
- **A slider** moves by a sideways drag anywhere on its row, from where it is, never jumping to the finger; a tap changes nothing, and a vertical move scrolls the panel.
- **A long press on a slider** opens darktable's fine control, the slider popup ([0042](0042-numbers-shown-typed-and-stored.md)), at touch size with its own keypad.
- **A list's options** preview as a finger slides across them and are chosen as it lifts, in place of the preview on hover.
- **A picker:** a tap picks a point, a drag an area, two fingers zoom and pan.
- **A long press** opens the context menus that exist ([0091](0091-context-menus-places-and-tools-items-as-actions.md)).
- **A swipe on the photo at Fit** does nothing; it never moves to another photo.
- **In portrait** the side panel becomes a bottom sheet.
- **The Photos tab** has a Select mode: a tap or a slide selects photos, with a batch bar for what acts on them.

### Rules for interface code, from now on

Standard is built now and Touch later, so these keep Touch an add-on rather than a rewrite. They hold for every component and panel:

1. **No hard-coded sizes in components.** Every height, width and gap comes from a theme token (`sizes.row`, `sizes.field`, `sizes.control` and the rest), so Touch mode is a second set of token values.
2. **Nothing reachable only by hover.** A tooltip may explain; it is never the only way to do or find something.
3. **No action only on a right-click, a double-click or a modifier key.** Each also has a visible control or a menu item. This extends [0091](0091-context-menus-places-and-tools-items-as-actions.md)'s rule for context menus to every gesture.
4. **Sliders and pickers only through their design-system components,** `SliderRow`, `PickerButton` and the like, never drawn by hand for one tool, so their touch behaviour is one change in one place.

## Options considered

- **A separate touch app,** designed for touch rather than adapted from the desktop layout: two designs of every tab and tool to keep in step, which a small project cannot afford, and a tablet user would learn a second interface.
- **Mouse and Touch as the names:** macOS is used with trackpads, which are no mouse.
- **Desktop and Tablet as the names:** they name a device, while the mode follows the input; a touch-screen laptop or a tablet with a keyboard and trackpad would be misnamed.
- **Pointer and Touch as the names:** accurate, but "Pointer" is jargon to most photographers; Standard says it is the usual one.
- **Automatic as the default on the desktop:** telling a touch screen with no pointer from one with a pointer is unreliable on Windows and Linux, and an interface that changes its own size surprises; Automatic stays a choice there.

## Consequences

- **Decision 35:** touch is a mode of the one egui interface, not a separate app; a phone's layout remains a later design.
- **Decision 1:** the mobile phase builds Touch mode for tablets and designs the phone's layout.
- **Decision 36:** the design system's sizes and spacing gain a second set of values for Touch mode (planned), and its record lists the four rules above.
- **Decision 91:** a long press opens the same context menus in Touch mode, and its rule that every item is reachable elsewhere extends to double-clicks and modifier keys.
- **Where the engine runs** on a tablet stays [0058](0058-a-native-engine-every-client-through-the-session-protocol.md)'s, a client over the session protocol, until the mobile phase is planned.
- **Code from now on:** reviews check the four rules; a component with a literal size, an action reached only by hover, right-click, double-click or a modifier, or a slider or picker drawn by hand is sent back. Where existing code breaks one, it is fixed as that code is next changed.
- **Planned:** `interface.mode` in the settings schema, `touch-mode` in the registry and View, Touch's token values, its gestures in the components, the bottom sheet and Photos' Select mode, all in the mobile phase.
- **docs:** [gui.md](../gui.md#touch-mode) describes Touch mode and the rules; [session-protocol.md](../session-protocol.md#web-tablets-and-remote-access) and [roadmap.md](../roadmap.md#mobile-after-phase-3) no longer plan a separate touch app.
- **The agent kit:** the `ui-conventions` skill needs the same four rules, a pull request to the kit once this lands.
