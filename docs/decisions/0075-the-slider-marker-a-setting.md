# 0075. The slider's marker, a setting

## Context

A slider row's marker, the handle on its track, was darktable's triangle, cut off at the track's ends ([0042](0042-numbers-shown-typed-and-stored.md)), and a theme could not change it ([0036](0036-the-design-system.md)). Cut in half at its minimum and maximum, a marker reads poorly, and photographers coming from commercial raw editors know a knob that stays whole at the ends. The marker's look is the interface's, not the processing's, so the migration's rule to mirror darktable does not bind it ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)).

## Decision

- **The marker is a setting, Slider marker** (`sliders.marker`, planned), with three shapes, each shown in Settings by its own picture:
  - **Triangle** (`triangle`): darktable's, 13 × 11 points, pointing at the value from below.
  - **Circle** (`circle`): 11 points across.
  - **Square** (`square`): 10 × 11 points, its corners rounded 2.5 points, a little taller than wide. **The default.**
- **Each is drawn alike:** centred on the track's middle, filled in `slider_handle` with a 1.5-point outline in `slider_handle_outline`, `slider_handle_hover` under the pointer, and while dragged an accent border just inside the outline (`slider_handle_active`), as the triangle has it.
- **The circle and the square gain a short centre line while dragged:** 1 point wide and 3.5 points long, in `slider_handle_active`, so they point at the value as a fader's cap does while it moves. At rest they are plain.
- **The marker stays whole at the track's ends.** The track still runs the content's full width; the marker's travel stops short of each end by half the shape's width where it crosses the track plus half its outline, so it sits flush with the end: 5 points for the triangle, 6.25 for the circle, 5.75 for the square. The fill from the default follows the same travel, so the marker always sits where the fill ends, and the track reads full at the maximum and empty at the minimum.
- **One marker everywhere a value is set on a track:** the slider rows, the central bar's scale and the curve's handles.
- **Values, keys, scripts and what is stored do not change:** only where the marker is drawn moves.

## Options considered

- **Darktable's triangle alone, cut off at the ends:** precise, and known to darktable's users, but half a marker at each end reads as broken.
- **Keeping the marker whole by moving it inside the track at the ends with the fill still spanning the full width:** the marker and the fill would disagree at the ends.
- **One stop for every shape:** each shape would leave a different gap at the ends.
- **The triangle or the circle as the default:** the triangle points most precisely, but the value is always written above the track; the circle reads as a knob but looks like the kept ● beside the label.
- **The shape as part of a theme:** a theme changes colours and sizes, not how controls behave; the marker is the user's own preference.

## Consequences

- `docs/gui.md`: the themes' list no longer fixes the handle as a triangle; the slider row, its layout and the curve describe the chosen marker and its stops; the slider popup's cards gain the marker as card 11.
- The settings schema gains `sliders.marker` with its three choices, and Settings › General shows it, when card 11 is built.
- `apertrust-ui` draws the three shapes, their hover and dragged looks and their stops, and the gallery's snapshots photograph each in both themes.
- The agent kit's `ui-conventions` skill, which describes the handle as the triangle's tip and base, changes to match once this lands.
