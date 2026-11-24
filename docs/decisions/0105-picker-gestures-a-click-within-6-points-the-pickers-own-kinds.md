# 0105. Picker gestures: a click within 6 points, the picker's own kinds

## Context

[0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md) gives the photo darktable's pointer gestures while a picker is on, and [0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md) makes the picker's cursor a second door to the same picker, so both doors share them. Three things about those gestures need settling: what separates a click from a drag, whether a picker can be switched between a point and an area on the photo, and how large the area a click makes is.

darktable 5.6.2, the reference ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)), works like this:

- **Each picker declares its kinds** (`DT_COLOR_PICKER_POINT`, `_AREA` or both).
  - Every picker that writes to the edit takes exactly one kind. White balance, Exposure, Colour calibration, Colour balance RGB, AgX and RGB curve's create curve take an area; RGB levels takes a point.
  - Every picker that takes both kinds only shows a value: the color picker utility, RGB curve's show colour, and later the colour equalizer's hue and blending's show colour.
- **Ctrl-click switches every picker in its code.** Ctrl-click on the photo, new in 5.6, switches point and area whatever the picker declares (`darkroom.c:4017-4034`). Its manual limits it to "modules where both modes are available".
- **A point is one pixel** (`color_picker.c:444-446`). Apertrust reads the raw stage at full resolution ([0063](0063-departures-from-darktable-and-views-below-full-scale.md)), so a point on White balance is one photosite, a single CFA colour, and its pick has no meaning.
- **Any motion starts a drag.** darktable follows the pointer after any motion (`mouse_moved`, `darkroom.c:3843-3858`). So a click with a point or two of jitter replaces the click's area with one a pixel wide. Apertrust's app does the same today.
- **A click on an area picker makes a 2 % area.** The area spans 2 % of the photo's width each way of the press (`darkroom.c:4064-4073`). That is a share of the photo, so at 800 % it covers far more than the patch pointed at.

## Decision

**A press on the photo is a click while the pointer stays within 6 points of where it went down, and a drag once it moves past them. The picker's declared kinds decide what a click and a drag make. A picker that takes one kind keeps it everywhere: on the photo and in the session.** Planned, every part; [pickers.md](../pickers.md#on-the-photo) holds the tables.

1. **The threshold** is 6 points on screen, egui's `max_click_dist`: a press that moves exactly 6 points is still a click, as egui counts a drag only past it.
   - It is measured by distance alone, however long the press is held, and never by egui's 0.8 s click time.
   - It applies to every picker from either door, to a corner's or a point's drag, and to the cursors' Click and Area gestures ([cursors.md](../cursors.md#on-the-photo)).
   - Until the press passes it, nothing follows the pointer: the click's area or point stays as the press sent it.
   - A press held still never resets the area. The picker ignores egui's long-press secondary click on a touch screen until Touch mode ([0094](0094-one-interface-standard-and-touch-modes.md)) settles touch.
2. **The kinds decide what a gesture makes** (`PickerKind`):
   - **An area picker** (White balance, Exposure's two, Colour calibration, Colour balance RGB, AgX, RGB curve's create curve):
     - a click makes the click's area round the press (rule 3);
     - a drag from empty photo draws an area;
     - a press within 6 points of a corner resizes the area from the opposite corner;
     - it never becomes a point.
   - **A point picker** (RGB levels' black, grey and white): a click sets the point and a drag moves it. It never becomes an area.
   - **A picker that takes both kinds** (the Colour picker, RGB curve's show colour; later the Colour equalizer's hue and blending's show colour):
     - a click picks a point;
     - a drag draws an area;
     - a press on the point's mark, within its reach, drags the point;
     - a press on a corner of the area shown resizes it.
3. **The click's area keeps a fixed size on screen.** It is the area's size at Fit, at every zoom: its half side is 2 % of the photo's width at Fit, in points, and it is as tall on screen.
   - The app works it out from the view and sends it as `Area`.
   - The session's `Region::pressed`, and with it `PickerCommand::Once`, keeps darktable's 2 % of the photo, so scripts pick as before.
   - At Fit the two are the same area.
4. **No switch on the photo.**
   - Ctrl-click (Cmd-click on the Mac) is a plain click carrying `PickKeys::command`, for every picker. Colour balance RGB's hue picker and RGB curve's create curve read that key.
   - Space on the focused picker switches nothing. The picker's node still keeps Space, so Space never moves to the next photo while picking.
   - The picker button still chooses the starting kind of a picker that takes both: right-click, Ctrl-click, Shift+F10 or the Menu key start it as an area.
   - For keyboard users, the Colour picker tool gets a Point | Area choice for its live sample, built with that tool (79.16).
5. **The session holds a picker to its declared kinds.** For a picker that takes one kind it refuses three commands: `Switch`, `Point` on an area picker, and `Area` on a point picker. Each is refused as `SessionError::Refused`, the reason `SessionError::OneKind { takes }` naming the kind the picker takes.
   - `Switch` keeps its meaning for a picker that takes both kinds.
   - `Once` is unchanged: it reads one region once and moves no picker's kind.
6. **A stray click on a picker that writes** replaces the area, and the area is redrawn by hand. Undo takes back the values only, as now, and turns the picker off as any change of its tool does ([0078](0078-undo-redo-and-the-edits-history.md)).
7. **The viewer bar's words follow the kinds,** in British English:

   | Picker | Message | Words |
   | --- | --- | --- |
   | An area picker | `picker-bar-area` | "Click for a small area, or drag one. Drag a corner to resize. Right-click resets the area. Esc ends." |
   | A point picker | `picker-bar-point` | "{ $what } Esc ends." |
   | Both kinds, measuring a point | `picker-bar-both-point` | "Click for a point, drag for an area. Esc ends." |
   | Both kinds, measuring an area | `picker-bar-both-area` | "Click for a point, drag for an area. Drag a corner to resize. Right-click resets the area. Esc ends." |

   `picker-bar-point-what` goes, since no area picker is ever a point.

## Options considered

- **darktable's code, Ctrl-click switching every picker:** faithful to the code, but a Ctrl-click turns White balance into one photosite and Exposure into one pixel's noise. Decision 79 already refused that reading for a one-shot click.
- **A switch that reads a point on an area picker as `Region::pressed`:** no trap, but Ctrl-click would then do almost nothing on most pickers.
- **Ctrl-click and Space kept as the switch on pickers that take both kinds:** brings back a lost area. It keeps a hidden modifier beside a gesture that already says which kind it wants, and these pickers write nothing, so nothing is lost to the edit.
- **darktable's sticky kind on pickers that take both:** area sampling, the right sample for skin, stays behind a modifier.
- **egui's own click rule, distance or 0.8 s:** a slow, careful click held still for longer than 0.8 s would be neither a click nor a drag.
- **No threshold, darktable's any-motion drag:** keeps today's fault, a click's area silently shrunk to a pixel or two by jitter.
- **darktable's 2 % of the photo at every zoom:** at 800 % on a photo 6000 pixels wide the area runs about 1900 points each way, far past the patch pointed at.
- **2 % capped to a share of the view:** two rules where one fixed size on screen is enough.
- **The session accepting `Switch` from a script on a picker that takes one kind:** a script could still put White balance into a point that reads nothing.
- **The session reading such a `Switch` or `Point` as `Region::pressed`:** a command that silently means something else. A refusal tells the script.
- **Undo restoring the picker's place and leaving it on:** an engine change to view state on Undo, for the first pass only.
- **A click moving the area while keeping its size:** departs from darktable's press. It is an open question for the UX pass after Phase 3 ([pickers.md](../pickers.md#open-questions)).

## Consequences

- **Departures from darktable.** These are first-pass departures ([0002](0002-rewrite-darktable-first-then-apertrusts-own-way.md)): the threshold, no switch on the photo, a click and a drag on pickers that take both kinds, and the click's area at a fixed size on screen. Each is listed in [pickers.md](../pickers.md#against-darktable).
- **[0079](0079-colour-pickers-one-api-for-tools-and-the-colour-picker.md)'s gestures** are darktable's under these rules. `Switch` is for a picker that takes both kinds, and the app's press makes its click's area from the view rather than `Region::pressed`.
- **[0104](0104-cursors-one-chosen-at-a-time-in-a-well-on-the-toolbar.md)'s Colour picker cursor** has no Ctrl-click switch. The cursors' Click and Area gestures share the threshold.
- **The app** (`crates/app/src/pickers.rs`, `on_photo` and `pressed_on`):
  - the threshold and the long-press rule;
  - the kinds' tables;
  - no `Switch` for a picker that takes one kind;
  - the click's area from the view;
  - the viewer bar's messages in every locale.
- **The engine** (`session/pickers.rs`): the refusal `SessionError::OneKind`.
- **`apertrust-tools`** gains a function that makes the click's area at a given scale beside `Region::pressed`, which stays as it is.
- **The plan** is in [pickers.md](../pickers.md#the-plan), 105.1 to 105.5:
  - the threshold first;
  - pickers that take one kind holding it, in the app and then in the session;
  - the click's area;
  - pickers that take both kinds with the Colour picker tool (79.16), RGB curve's show colour following with its pick (79.12).
