# 0081. Icon-only buttons draw no plates; On is the filled icon

## Context

Buttons with words show hover and press on a plate (`button`, `button_hover`, `button_pressed`), and the bars' quiet buttons show one only while touched. A module's header has icon buttons 14 points wide, a control high, 4 points apart, and a plate that narrow under the pointer reads as a sliver, not a button; a plate or a box for On beside plate-free neighbours reads as a different control. The question is how an icon-only button shows hover, press and On, the same way everywhere.

## Decision

**An icon-only button (`IconButton`) draws no plate in any state; its icon says it all.** Buttons with words keep their plates: text buttons, the bars' quiet `ToolButton`s.

| State | The icon |
| --- | --- |
| Rest | The outline icon in `text_secondary` |
| Under the pointer | The same icon in `text_strong`; high contrast also draws its 1-point `button_edge` outline round the button, as its other controls do, and keeps it while the button is held down |
| Held down | The icon in `accent_text` (dark #FFB833, grey #FFC766, light #8F5A00, high contrast #FFBF00), while pressed only: feedback, never a state |
| On | The icon's filled form in `text_strong`: a toggle that is on, a picker while picking, a quad's picker, a button whose menu is open |
| Focused by a key | As at rest, with the focus ring |
| Disabled | `text_disabled` |

- **Filled forms.** Lucide draws outlines, so each icon an icon button shows On has a filled form, its closed shapes filled in the icon's colour (`Icon::filled`; `crates/ui/assets/icons`): the central bar's toggles, the grid bar's Labels, and the quads' pipette and line (`pick-filled`, `line-filled`). An icon of lines and dots alone has no shape to fill, so it shows On by `text_strong` alone, as under the pointer: ⋯, ⋮, », ☰ (the menu and presets), ＋ and Sort's arrows, each of which opens a menu, and the menu hanging from it shows that it is open. Every toggle has a filled form; a button that opens a menu tells screen readers whether it is open (`pull_down`).
- **The hit area** stays the whole button; nothing grows on hover.
- The rule is the wireframe's "Controls · icon-only buttons: no plates, the filled icon for On" board. Built in `IconButton` (`crates/ui/src/components/icon_button.rs`).

## Options considered

- **A square plate on hover,** as wide as the button is high: the header's buttons would overlap or need wider gaps, and the plate would still be the only cue many places draw.
- **A plate for On** (`raised` with a hairline, as before): a box beside plate-free neighbours, and two cues for one state where the filled shape already carries it.
- **An option between the two styles:** one rule keeps the plate-free look consistent across the app, and an option would double every state to draw, test and document.

## Consequences

- `button`, `button_hover` and `button_pressed` stay for buttons with words; `raised` is no longer drawn behind an icon button.
- A new toggle's icon needs a filled form, or the reason it has none; `Icon::filled` maps it, and the icon rules (`crates/ui/assets/icons/README.md`) hold its file to its outline.
- In grey and high contrast, hover brightens little (1.4:1 from `text_secondary` to `text_strong`); high contrast adds its outline, grey relies on the colour.
- Icon buttons drawn apart from `IconButton`, the jobs spinner and the scopes' overlay buttons among them, keep their own looks until they are moved onto it.
- `docs/gui.md` (Checkboxes, text buttons and icon buttons; Quieter icons; the tokens) describes the rule, and the `ui-conventions` skill is to describe it.
